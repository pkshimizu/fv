//! **Copy Plan**（CONTEXT.md 参照）の構築と実行単位。
//! Scan Phase が source tree を走査して `CopyPlan`（作成すべきディレクトリ列＋コピーすべき
//! エントリ列）を組み立て、Operation Phase は `copy_entry` で 1 エントリずつ実行する。
//! Copy と Move（EXDEV フォールバック）が共通利用する。

use super::checkpoint::{CollectStatus, notify_scan_progress};
use super::destination::{TopLevelPair, resolve_top_level_pairs};
use crate::fs::VFile;
use crate::state::Phase;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// 1 件分の `CopyEntry` を宛先に書き出す。
/// 通常ファイル/file-symlink は `std::fs::copy` でリンク先のデータをコピーするが、
/// symlink (特に dir-symlink) は `std::os::unix::fs::symlink` でリンク自体を再生成する。
/// これにより macOS の `.app` バンドル等に含まれる `Resources -> Versions/A/Resources` のような
/// dir-symlink を含むツリーを `cp -R` と同等に正しくコピーできる。
pub(super) fn copy_entry(entry: &CopyEntry) -> Result<()> {
    match entry {
        CopyEntry::File { src, dst } => {
            std::fs::copy(src, dst)
                .with_context(|| format!("{}: Failed to copy file", dst.display()))?;
            Ok(())
        }
        #[cfg(unix)]
        CopyEntry::Symlink { dst, target } => {
            std::os::unix::fs::symlink(target, dst)
                .with_context(|| format!("{}: Failed to create symlink", dst.display()))?;
            Ok(())
        }
    }
}

/// Scan Phase が組み立てる Copy の実行計画。
/// `directories`: 作成すべき宛先ディレクトリ列 (親が子に先行する DFS pre-order)。
///     単一ファイル root では何も追加されず空のまま (mkdir は `run_copy` 冒頭の `create_dir_all(dest)` で十分)。
/// `files`: コピーすべきエントリ列 (通常ファイル + symlink)。各 dst の親は `directories` に含まれるか
///     `dest` 自体のため、Operation Phase ではディレクトリ作成不要。
#[derive(Debug, Default)]
pub(super) struct CopyPlan {
    pub(super) directories: Vec<PathBuf>,
    pub(super) files: Vec<CopyEntry>,
}

/// Scan Phase で 1 エントリ分の処理計画を保持する。
/// 通常ファイル/file-symlink は `File` バリアントで `std::fs::copy` 経由のデータコピー、
/// それ以外の symlink (主に dir-symlink) は `Symlink` バリアントで再生成する。
#[derive(Debug)]
pub(super) enum CopyEntry {
    File {
        src: PathBuf,
        dst: PathBuf,
    },
    #[cfg(unix)]
    Symlink {
        dst: PathBuf,
        /// `std::fs::read_link` が返した値をそのまま保持 (相対パスは相対のまま再生成され、
        /// macOS の `.app` バンドルのような相対 symlink 構造を壊さない)。
        target: PathBuf,
    },
}

/// `roots` 配下を一度走査して、cancel 可能なまま `CopyPlan` を組み立てる (Scan Phase)。
///
/// - `Ok(Some(plan))`: 全 root を列挙完了。`plan.files` の各 src→dst をコピーすれば結果が得られる
/// - `Ok(None)`: Scan 中に Cancel Token がセットされ早期中断。Partial Result は無し
/// - `Err`: 走査中の I/O エラー (`read_dir` 失敗、`unique_path` 失敗など)
///
/// 各 root の top-level 名は `pick_unique_top_dest` で衝突回避し、`dest/<name>` がすでに
/// 存在するか、**同一 batch 内で既に他 root に予約されている** 場合は `<name>_1`, `<name>_2`, ... に
/// 振り替える (旧 `fs::file::copy_to` と同じ規約 + multi-root batch での内部衝突回避)。
pub(super) fn scan_copy_plan(
    roots: &[VFile],
    dest: &Path,
    exact_path: bool,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<Option<CopyPlan>> {
    let pairs = resolve_top_level_pairs(roots, dest, exact_path)?;
    scan_pairs_into_plan(&pairs, cancel, on_progress)
}

/// 事前解決済みの `TopLevelPair` 列を走査して `CopyPlan` を組み立てる Scan Phase。
/// Copy/Move の EXDEV フォールバックで共通利用する。
pub(super) fn scan_pairs_into_plan(
    pairs: &[TopLevelPair],
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<Option<CopyPlan>> {
    let mut plan = CopyPlan::default();
    on_progress(Phase::Scanning, 0, None);
    for pair in pairs {
        if cancel.load(Ordering::Relaxed) {
            return Ok(None);
        }
        // top-level は旧 fs::file::copy_to と同じく metadata (symlink follow) で判定する。
        // ユーザがコマンドで明示的に指定した対象なので、dir-symlink ならその内容をコピー。
        // `Path::is_dir()` ではなく `metadata()?` を使うことで、stat 失敗を「通常ファイル扱い」に
        // 握りつぶさず Scan Phase で早期 Err として顕在化する。
        let metadata = pair
            .src
            .metadata()
            .with_context(|| format!("{}: Failed to stat source", pair.src.display()))?;
        let status = if metadata.is_dir() {
            collect_directory_into_plan(&pair.src, &pair.dst, &mut plan, cancel, on_progress)?
        } else {
            enqueue_entry(
                &mut plan,
                CopyEntry::File {
                    src: pair.src.clone(),
                    dst: pair.dst.clone(),
                },
                on_progress,
            );
            CollectStatus::Completed
        };
        match status {
            CollectStatus::Completed => {}
            CollectStatus::Cancelled => return Ok(None),
        }
    }
    Ok(Some(plan))
}

/// `src` ディレクトリ配下を pre-order DFS で plan に積む。
/// `src` は呼び出し側で `metadata().is_dir()` 判定済み (top-level は symlink follow 結果として
/// dir-symlink である可能性あり、再帰内はリンクをたどっていないので非 symlink のディレクトリ)。
/// `read_dir` で得たエントリの `file_type()` を使い、ディレクトリでもシンボリックリンクは
/// たどらない (dir-symlink ループや任意領域脱出の防止)。
fn collect_directory_into_plan(
    src: &Path,
    dst: &Path,
    plan: &mut CopyPlan,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<CollectStatus> {
    plan.directories.push(dst.to_path_buf());
    for entry in std::fs::read_dir(src)
        .with_context(|| format!("{}: Failed to read directory", src.display()))?
    {
        // File-level Checkpoint: 各エントリ処理の前に cancel をチェックする
        // (ZipExtract の `for i in 0..total` と対称形)。
        if cancel.load(Ordering::Relaxed) {
            return Ok(CollectStatus::Cancelled);
        }
        let entry =
            entry.with_context(|| format!("{}: Failed to read directory entry", src.display()))?;
        let entry_src = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("{}: Failed to read file type", entry_src.display()))?;
        let entry_dst = dst.join(entry.file_name());
        if file_type.is_dir() && !file_type.is_symlink() {
            match collect_directory_into_plan(&entry_src, &entry_dst, plan, cancel, on_progress)? {
                CollectStatus::Completed => {}
                CollectStatus::Cancelled => return Ok(CollectStatus::Cancelled),
            }
        } else if cfg!(unix) && file_type.is_symlink() {
            // symlink (file-symlink / dir-symlink いずれも) はリンク自体を再生成する。
            // dir-symlink を `std::fs::copy` でたどると "Is a directory" でエラーになるため、
            // また file-symlink でもリンク先データを書き出すと bundle 構造が壊れるため、
            // 一律 `read_link` で target を取得して `std::os::unix::fs::symlink` で再生成する。
            #[cfg(unix)]
            {
                let target = std::fs::read_link(&entry_src).with_context(|| {
                    format!("{}: Failed to read symlink target", entry_src.display())
                })?;
                enqueue_entry(
                    plan,
                    CopyEntry::Symlink {
                        dst: entry_dst,
                        target,
                    },
                    on_progress,
                );
            }
        } else {
            // 通常ファイル・特殊ファイルは std::fs::copy で内容コピーする。
            // (cfg(not(unix)) では symlink もこの分岐に落ちる。Windows での symlink 再生成は
            // 別 API のため未対応で、既存挙動 - リンク先データのコピー試行 - を維持する。)
            enqueue_entry(
                plan,
                CopyEntry::File {
                    src: entry_src,
                    dst: entry_dst,
                },
                on_progress,
            );
        }
    }
    Ok(CollectStatus::Completed)
}

/// plan.files に 1 件積み、SCAN_NOTIFY_BATCH 件ごとに Scanning 進捗を通知するヘルパ。
/// ファイル発見ごとの per-iteration callback コストを抑える。
fn enqueue_entry(
    plan: &mut CopyPlan,
    entry: CopyEntry,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) {
    plan.files.push(entry);
    notify_scan_progress(plan.files.len(), on_progress);
}
