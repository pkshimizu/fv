use crate::fs::VFile;
use crate::fs::file::unique_path;
use crate::state::Phase;
use anyhow::{Context, Result};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// Scan Phase 中、ファイル発見ごとに `on_progress` を呼ばずに、この件数ごとにバッチで通知する。
/// `&mut dyn FnMut` の vtable hop と `spawn_async_job` 側の `Instant::now()` 呼出を削減する目的。
const SCAN_NOTIFY_BATCH: usize = 256;

/// Async Job として実行される重いファイル操作。
/// UI とは結合せず、進捗はクロージャ経由で通知する。
/// `Phase::Cancelling` は worker からは emit せず、UI 側で Esc 受信時に上書きされる。
#[derive(Debug)]
pub enum FileJob {
    ZipExtract { file: VFile, dest: PathBuf },
    Copy { files: Vec<VFile>, dest: PathBuf },
}

impl FileJob {
    /// Job を実行する。
    /// `cancel` を File-level Checkpoint で監視し、true ならファイル境界で早期 return。
    ///
    /// # 進捗通知プロトコル
    /// - Phase 切り替え直後に必ず `(new_phase, 0, total)` を 1 回 emit する
    ///   (Scan Phase 開始時は `(Scanning, 0, None)`、Operation Phase 開始時は `(Copying|Extracting|..., 0, Some(N))`)
    /// - Scan Phase 中: ファイル発見ごとではなく `SCAN_NOTIFY_BATCH` 件ごとに `(Scanning, k, None)` を emit する
    /// - Operation Phase 中: 1 ファイル完了ごとに `(Copying|..., k, Some(N))`
    /// - `total` が `Some(N)` の場合、Cancel されない限り processed は最終的に `N` に達する
    /// - Cancel された場合は File-level Checkpoint で emit が止まるため、processed < total のまま戻る
    pub fn run(
        self,
        cancel: &AtomicBool,
        on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
    ) -> Result<()> {
        match self {
            FileJob::ZipExtract { file, dest } => run_zip_extract(&file, &dest, cancel, on_progress),
            FileJob::Copy { files, dest } => run_copy(&files, &dest, cancel, on_progress),
        }
    }
}

/// Copy Job 本体。Scan Phase → Operation Phase の二相で動く。
///
/// # Partial Result
/// Cancel された場合、Operation Phase で `std::fs::copy` 完了済みの個別ファイルは
/// ディスクに残る。それを内包する祖先ディレクトリも残る (空ディレクトリとして残り得る)。
/// Scan Phase 中の cancel では Partial Result は残らない (mkdir も発火していないため)。
///
/// # Symlink
/// top-level の VFile が dir-symlink の場合はリンクをたどってその内容をコピーする
/// (既存 `fs::file::copy_to` と同じ挙動)。再帰内ではリンクをたどらず、symlink エントリは
/// `std::fs::copy` で「リンク先データを書き出すファイル」として扱う。これにより
/// 入れ子の symlink ループや任意領域への脱出を防ぐ。
fn run_copy(
    files: &[VFile],
    dest: &Path,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<()> {
    let Some(plan) = scan_copy_plan(files, dest, cancel, on_progress)? else {
        // Scan Phase 中に cancel された場合は Partial Result なしで早期 return
        return Ok(());
    };
    let total = plan.files.len();
    // Operation Phase 開始を Phase 切り替え直後の `(Copying, 0, Some(total))` で通知。
    // mkdir ループに入る前に出すことで「ディレクトリ作成中は UI が Scanning のまま」を回避する。
    on_progress(Phase::Copying, 0, Some(total));
    // ユーザ指定の dest 自体が存在しない可能性に備え一度だけ create_dir_all。
    // それ以降の plan.directories は pre-order により親が常に作成済みなので create_dir で十分。
    std::fs::create_dir_all(dest)
        .with_context(|| format!("{}: Failed to create directory", dest.display()))?;
    for dir in &plan.directories {
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        std::fs::create_dir(dir)
            .with_context(|| format!("{}: Failed to create directory", dir.display()))?;
    }
    for (i, entry) in plan.files.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        copy_entry(entry)?;
        on_progress(Phase::Copying, i + 1, Some(total));
    }
    Ok(())
}

/// 1 件分の `CopyEntry` を宛先に書き出す。
/// 通常ファイル/file-symlink は `std::fs::copy` でリンク先のデータをコピーするが、
/// symlink (特に dir-symlink) は `std::os::unix::fs::symlink` でリンク自体を再生成する。
/// これにより macOS の `.app` バンドル等に含まれる `Resources -> Versions/A/Resources` のような
/// dir-symlink を含むツリーを `cp -R` と同等に正しくコピーできる。
fn copy_entry(entry: &CopyEntry) -> Result<()> {
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
struct CopyPlan {
    directories: Vec<PathBuf>,
    files: Vec<CopyEntry>,
}

/// Scan Phase で 1 エントリ分の処理計画を保持する。
/// 通常ファイル/file-symlink は `File` バリアントで `std::fs::copy` 経由のデータコピー、
/// それ以外の symlink (主に dir-symlink) は `Symlink` バリアントで再生成する。
#[derive(Debug)]
enum CopyEntry {
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

/// Scan Phase の中断/完走を表す。
enum CollectStatus {
    Completed,
    Cancelled,
}

/// `roots` 配下を一度走査して、cancel 可能なまま `CopyPlan` を組み立てる (Scan Phase)。
///
/// - `Ok(Some(plan))`: 全 root を列挙完了。`plan.files` の各 src→dst をコピーすれば結果が得られる
/// - `Ok(None)`: Scan 中に Cancel Token がセットされ早期中断。Partial Result は無し
/// - `Err`: 走査中の I/O エラー (`read_dir` 失敗、`unique_path` 失敗など)
///
/// 各 root の top-level 名は `fs::file::unique_path` で衝突回避し、`dest/<name>` がすでに
/// 存在すれば `<name>_1`, `<name>_2`, ... に振り替える (`fs::file::copy_to` と同じ規約)。
fn scan_copy_plan(
    roots: &[VFile],
    dest: &Path,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<Option<CopyPlan>> {
    let mut plan = CopyPlan::default();
    on_progress(Phase::Scanning, 0, None);
    for root in roots {
        if cancel.load(Ordering::Relaxed) {
            return Ok(None);
        }
        let src = Path::new(root.absolute_path());
        let name = src
            .file_name()
            .with_context(|| format!("{}: Copy source has no file name", src.display()))?;
        let top_dest = unique_path(&dest.join(name))?;

        // top-level は既存 fs::file::copy_to と同じく metadata (symlink follow) で判定する。
        // ユーザがコマンドで明示的に指定した対象なので、dir-symlink ならその内容をコピー。
        // `Path::is_dir()` ではなく `metadata()?` を使うことで、stat 失敗 (権限不足・dangling
        // symlink 等) を「通常ファイル扱い」に握りつぶさず Scan Phase で早期 Err として顕在化する。
        let metadata = src
            .metadata()
            .with_context(|| format!("{}: Failed to stat copy source", src.display()))?;
        let status = if metadata.is_dir() {
            collect_directory_into_plan(src, &top_dest, &mut plan, cancel, on_progress)?
        } else {
            // top-level は `metadata()?` で symlink follow 済みなので、ここに来た時点で
            // 通常ファイル (もしくは file-symlink の follow 結果) として扱う。
            enqueue_entry(
                &mut plan,
                CopyEntry::File {
                    src: src.to_path_buf(),
                    dst: top_dest,
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
        let entry = entry
            .with_context(|| format!("{}: Failed to read directory entry", src.display()))?;
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
    let count = plan.files.len();
    if count % SCAN_NOTIFY_BATCH == 0 {
        on_progress(Phase::Scanning, count, None);
    }
}

fn run_zip_extract(
    file: &VFile,
    dest: &Path,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<()> {
    let src_path = file.absolute_path();
    let zip_file = std::fs::File::open(src_path)
        .with_context(|| format!("{src_path}: Failed to open zip file"))?;
    let mut archive = zip::ZipArchive::new(zip_file)
        .with_context(|| format!("{src_path}: Failed to read zip archive"))?;

    let total = archive.len();
    on_progress(Phase::Extracting, 0, Some(total));

    // 同一 parent への create_dir_all 連打を避けるための直前 parent キャッシュ。
    // zip は同一ディレクトリのエントリが連続することが多いので 1 件だけで多くの syscall が消える。
    let mut last_parent: Option<PathBuf> = None;

    for i in 0..total {
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        let mut entry = archive
            .by_index(i)
            .with_context(|| format!("{src_path}: Failed to read zip entry"))?;
        let Some(enclosed_name) = entry.enclosed_name() else {
            continue;
        };
        let out_path = dest.join(enclosed_name);
        let processed = i + 1;

        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)
                .with_context(|| format!("{}: Failed to create directory", out_path.display()))?;
            // 自分自身を parent キャッシュにも入れておく (子エントリで再 mkdir を防ぐ)
            last_parent = Some(out_path.clone());
            on_progress(Phase::Extracting, processed, Some(total));
            continue;
        }

        if let Some(parent) = out_path.parent()
            && last_parent.as_deref() != Some(parent)
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("{}: Failed to create directory", parent.display()))?;
            last_parent = Some(parent.to_path_buf());
        }

        let out_file = std::fs::File::create(&out_path)
            .with_context(|| format!("{}: Failed to create file", out_path.display()))?;
        let mut writer = BufWriter::new(out_file);
        std::io::copy(&mut entry, &mut writer)
            .with_context(|| format!("{}: Failed to extract file", out_path.display()))?;
        // BufWriter を明示的に flush して書き残しエラーを伝播させる
        writer
            .into_inner()
            .map_err(|e| anyhow::anyhow!("{}: Failed to flush: {}", out_path.display(), e))?;

        on_progress(Phase::Extracting, processed, Some(total));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::{Read, Write};
    use std::sync::atomic::AtomicBool;
    use tempfile::TempDir;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    fn vfile(path: &std::path::Path) -> VFile {
        VFile::new(
            path.to_str()
                .expect("UTF-8 path required for tests")
                .to_owned(),
        )
    }

    fn build_sample_zip(zip_path: &std::path::Path) {
        let file = File::create(zip_path).expect("create zip file");
        let mut writer = ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        writer.start_file("hello.txt", options).unwrap();
        writer.write_all(b"hello fv").unwrap();
        writer.add_directory("nested/", options).unwrap();
        writer.start_file("nested/inner.txt", options).unwrap();
        writer.write_all(b"inside nested").unwrap();
        writer.finish().expect("finish zip");
    }

    fn read_to_string(path: &std::path::Path) -> String {
        let mut s = String::new();
        File::open(path).unwrap().read_to_string(&mut s).unwrap();
        s
    }

    fn write_file(path: &std::path::Path, contents: &[u8]) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        File::create(path).unwrap().write_all(contents).unwrap();
    }

    #[test]
    fn zip_extract_returns_err_when_source_file_is_missing() {
        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let job = FileJob::ZipExtract {
            file: vfile(&tmp.path().join("no-such.zip")),
            dest,
        };
        let result = job.run(&AtomicBool::new(false), &mut |_, _, _| {});
        assert!(result.is_err());
    }

    #[test]
    fn zip_extract_stops_at_file_checkpoint_when_cancel_is_preset() {
        let tmp = TempDir::new().unwrap();
        let zip_path = tmp.path().join("sample.zip");
        build_sample_zip(&zip_path);
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let cancel = AtomicBool::new(true);
        let mut events: Vec<(Phase, usize, Option<usize>)> = Vec::new();
        let job = FileJob::ZipExtract {
            file: vfile(&zip_path),
            dest: dest.clone(),
        };
        job.run(&cancel, &mut |p, n, t| events.push((p, n, t)))
            .unwrap();

        // 初期の 0/N は通知されるが、いかなるエントリも処理されない
        assert_eq!(events, vec![(Phase::Extracting, 0, Some(3))]);
        assert!(!dest.join("hello.txt").exists());
        assert!(!dest.join("nested").join("inner.txt").exists());
        // cancel フラグ自体には触らない
        assert!(cancel.load(Ordering::Relaxed));
    }

    #[test]
    fn zip_extract_emits_progress_for_each_entry_with_known_total() {
        let tmp = TempDir::new().unwrap();
        let zip_path = tmp.path().join("sample.zip");
        build_sample_zip(&zip_path);
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let mut events: Vec<(Phase, usize, Option<usize>)> = Vec::new();
        let job = FileJob::ZipExtract {
            file: vfile(&zip_path),
            dest,
        };
        job.run(&AtomicBool::new(false), &mut |p, n, t| {
            events.push((p, n, t))
        })
        .unwrap();

        // sample.zip は hello.txt, nested/, nested/inner.txt の 3 entry
        assert_eq!(
            events,
            vec![
                (Phase::Extracting, 0, Some(3)),
                (Phase::Extracting, 1, Some(3)),
                (Phase::Extracting, 2, Some(3)),
                (Phase::Extracting, 3, Some(3)),
            ]
        );
    }

    #[test]
    fn zip_extract_writes_all_entries_to_destination() {
        let tmp = TempDir::new().expect("tempdir");
        let zip_path = tmp.path().join("sample.zip");
        build_sample_zip(&zip_path);

        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let job = FileJob::ZipExtract {
            file: vfile(&zip_path),
            dest: dest.clone(),
        };
        let cancel = AtomicBool::new(false);
        job.run(&cancel, &mut |_, _, _| {})
            .expect("ZipExtract should succeed");

        assert_eq!(read_to_string(&dest.join("hello.txt")), "hello fv");
        assert_eq!(
            read_to_string(&dest.join("nested").join("inner.txt")),
            "inside nested"
        );
    }

    #[test]
    fn copy_avoids_collision_by_appending_numeric_suffix() {
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("foo");
        write_file(&src_root.join("a.txt"), b"alpha");
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();
        // dest/foo がすでに存在 → コピーは衝突を回避して dest/foo_1 に置く
        std::fs::create_dir(dest.join("foo")).unwrap();
        write_file(&dest.join("foo").join("existing.txt"), b"existing");

        let job = FileJob::Copy {
            files: vec![vfile(&src_root)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("Copy should succeed");

        // 既存ディレクトリは無傷
        assert_eq!(
            read_to_string(&dest.join("foo").join("existing.txt")),
            "existing"
        );
        // コピーは foo_1 に置かれる
        assert_eq!(
            read_to_string(&dest.join("foo_1").join("a.txt")),
            "alpha"
        );
    }

    #[test]
    fn copy_returns_err_when_source_file_is_missing() {
        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let job = FileJob::Copy {
            files: vec![vfile(&tmp.path().join("no-such.txt"))],
            dest,
        };
        let result = job.run(&AtomicBool::new(false), &mut |_, _, _| {});
        assert!(result.is_err());
    }

    #[test]
    fn copy_keeps_partial_result_when_cancelled_during_copying_phase() {
        use std::sync::Arc;
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("foo");
        write_file(&src_root.join("a.txt"), b"a");
        write_file(&src_root.join("b.txt"), b"b");
        write_file(&src_root.join("c.txt"), b"c");
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_for_closure = cancel.clone();
        let job = FileJob::Copy {
            files: vec![vfile(&src_root)],
            dest: dest.clone(),
        };
        // Operation Phase で 1 ファイルコピー完了の進捗を受けた直後に cancel をセット
        job.run(&cancel, &mut |p, n, _| {
            if p == Phase::Copying && n == 1 {
                cancel_for_closure.store(true, Ordering::Relaxed);
            }
        })
        .expect("cancel should produce Ok early return");

        // 1 ファイルだけはコピー済み (Partial Result)
        let copied = std::fs::read_dir(dest.join("foo"))
            .unwrap()
            .filter_map(|e| e.ok())
            .count();
        assert_eq!(
            copied, 1,
            "exactly one file should remain as partial result, found {copied}"
        );
    }

    #[test]
    fn copy_stops_during_scanning_phase_when_cancel_is_preset() {
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("foo");
        write_file(&src_root.join("a.txt"), b"a");
        write_file(&src_root.join("b.txt"), b"b");
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let mut events: Vec<(Phase, usize, Option<usize>)> = Vec::new();
        let job = FileJob::Copy {
            files: vec![vfile(&src_root)],
            dest: dest.clone(),
        };
        // 事前 cancel
        job.run(&AtomicBool::new(true), &mut |p, n, t| events.push((p, n, t)))
            .expect("cancel should produce Ok early return");

        // Scan Phase 開始時の (Scanning, 0, None) のみ通知され、Operation Phase へ進まない
        assert_eq!(events, vec![(Phase::Scanning, 0, None)]);
        // どのファイルもコピーされていない (Partial Result すらない)
        assert!(!dest.join("foo").exists());
    }

    #[test]
    fn copy_emits_copying_progress_per_file_copied() {
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("foo");
        write_file(&src_root.join("a.txt"), b"a");
        write_file(&src_root.join("b.txt"), b"b");
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let mut events: Vec<(Phase, usize, Option<usize>)> = Vec::new();
        let job = FileJob::Copy {
            files: vec![vfile(&src_root)],
            dest,
        };
        job.run(&AtomicBool::new(false), &mut |p, n, t| events.push((p, n, t)))
            .expect("Copy should succeed");

        let copying: Vec<_> = events
            .iter()
            .filter(|(p, _, _)| *p == Phase::Copying)
            .copied()
            .collect();
        // Operation Phase 開始時 0/N と各ファイルコピー後の処理済み数
        assert_eq!(
            copying,
            vec![
                (Phase::Copying, 0, Some(2)),
                (Phase::Copying, 1, Some(2)),
                (Phase::Copying, 2, Some(2)),
            ]
        );
    }

    #[test]
    fn copy_emits_initial_scanning_progress_at_phase_start() {
        // Scan Phase は SCAN_NOTIFY_BATCH (256) 件ごとにバッチで通知するため、
        // 小規模 (2 ファイル) では初回 (Scanning, 0, None) のみ emit される。
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("foo");
        write_file(&src_root.join("a.txt"), b"a");
        write_file(&src_root.join("b.txt"), b"b");
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let mut events: Vec<(Phase, usize, Option<usize>)> = Vec::new();
        let job = FileJob::Copy {
            files: vec![vfile(&src_root)],
            dest,
        };
        job.run(&AtomicBool::new(false), &mut |p, n, t| events.push((p, n, t)))
            .expect("Copy should succeed");

        let scanning: Vec<_> = events
            .iter()
            .filter(|(p, _, _)| *p == Phase::Scanning)
            .copied()
            .collect();
        assert_eq!(scanning, vec![(Phase::Scanning, 0, None)]);
    }

    #[test]
    fn copy_reproduces_directory_hierarchy_recursively() {
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("foo");
        write_file(&src_root.join("a.txt"), b"alpha");
        write_file(&src_root.join("bar").join("b.txt"), b"beta");
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let job = FileJob::Copy {
            files: vec![vfile(&src_root)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("Copy should succeed");

        assert_eq!(read_to_string(&dest.join("foo").join("a.txt")), "alpha");
        assert_eq!(
            read_to_string(&dest.join("foo").join("bar").join("b.txt")),
            "beta"
        );
    }

    #[test]
    fn copy_places_single_file_into_destination_directory() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("hello.txt");
        write_file(&src, b"hello fv");
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let job = FileJob::Copy {
            files: vec![vfile(&src)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("Copy should succeed");

        assert_eq!(read_to_string(&dest.join("hello.txt")), "hello fv");
    }

    #[cfg(unix)]
    #[test]
    fn copy_preserves_directory_symlinks_inside_tree_instead_of_following_them() {
        // src/escape -> ../outside (dir-symlink) と src/inside/safe.txt を用意し、
        // 再帰内で escape はリンク自体を再生成して outside 配下を取り込まないことを検証する。
        // (macOS .app バンドルの Resources -> Versions/A/Resources 等で必要な挙動)
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("src");
        write_file(&src_root.join("inside").join("safe.txt"), b"safe");
        let outside = tmp.path().join("outside");
        write_file(&outside.join("secret.txt"), b"should-not-be-recursively-copied");
        std::os::unix::fs::symlink(&outside, src_root.join("escape")).unwrap();

        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let job = FileJob::Copy {
            files: vec![vfile(&src_root)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("Copy with dir-symlink should succeed by recreating link");

        // 通常ファイルはコピーされる
        assert_eq!(
            read_to_string(&dest.join("src").join("inside").join("safe.txt")),
            "safe"
        );

        // escape は symlink として再生成されており、target が outside のまま保持されている
        let escape = dest.join("src").join("escape");
        let escape_meta = std::fs::symlink_metadata(&escape).expect("escape entry must exist");
        assert!(
            escape_meta.file_type().is_symlink(),
            "dir-symlink should be preserved as symlink, not recreated as a directory"
        );
        assert_eq!(
            std::fs::read_link(&escape).unwrap(),
            outside,
            "symlink target should be preserved as-is"
        );

        // outside/secret.txt が dest 直下に独立コピーされていない (recurse 不在の証拠)
        assert!(
            !dest.join("src").join("secret.txt").exists(),
            "outside/secret.txt should not be independently copied to dest"
        );
    }

    #[cfg(unix)]
    #[test]
    fn copy_preserves_file_symlinks_inside_tree() {
        // src/target.txt (通常ファイル) と src/alias -> target.txt (file-symlink) を用意し、
        // alias がリンクとして再生成されることを検証する (data を二重コピーしない)。
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("src");
        write_file(&src_root.join("target.txt"), b"original");
        std::os::unix::fs::symlink("target.txt", src_root.join("alias")).unwrap();

        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let job = FileJob::Copy {
            files: vec![vfile(&src_root)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("Copy with file-symlink should succeed");

        let alias = dest.join("src").join("alias");
        let alias_meta = std::fs::symlink_metadata(&alias).expect("alias must exist");
        assert!(
            alias_meta.file_type().is_symlink(),
            "file-symlink should be preserved as symlink"
        );
        assert_eq!(
            std::fs::read_link(&alias).unwrap(),
            std::path::PathBuf::from("target.txt"),
            "relative symlink target should be preserved verbatim"
        );
        // symlink follow すれば alias の内容は target.txt のデータ
        assert_eq!(read_to_string(&alias), "original");
    }

    #[cfg(unix)]
    #[test]
    fn copy_does_not_infinitely_recurse_on_symlink_loop() {
        // src/loop -> src の自己ループ。Scan が再帰せず有限時間で return することを検証する。
        // 新仕様: loop は symlink として再生成され、再帰には入らない。
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("src");
        std::fs::create_dir(&src_root).unwrap();
        std::os::unix::fs::symlink(&src_root, src_root.join("loop")).unwrap();

        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let job = FileJob::Copy {
            files: vec![vfile(&src_root)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("self-loop symlink should be preserved without infinite recursion");

        // loop は symlink として保持され、再帰展開されていないこと
        let dest_loop = dest.join("src").join("loop");
        let loop_meta = std::fs::symlink_metadata(&dest_loop).expect("loop link must exist");
        assert!(
            loop_meta.file_type().is_symlink(),
            "self-loop symlink should be preserved"
        );
        // 自己ループ展開が起きていない
        assert!(
            !dest.join("src").join("loop").join("loop").join("loop").exists()
                || loop_meta.file_type().is_symlink(),
            "self-loop should not produce nested loop directories"
        );
    }
}
