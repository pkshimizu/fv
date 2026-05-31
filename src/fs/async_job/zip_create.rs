//! Zip 作成 Job。Scan Phase で Copy Plan 相当の Zip エントリ列を集め、Operation Phase で書き出す。
//! 途中終了時は ZipPathGuard の Drop で書きかけ .zip を削除する（ADR-0001 の Zip 例外）。

use super::checkpoint::{
    CollectStatus, SCAN_NOTIFY_BATCH, for_each_until_cancelled, process_items,
};
use super::destination::pick_unique_top_dest;
use crate::fs::VFile;
use crate::state::Phase;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// Zip 作成 Job 本体。Scan Phase → Operation Phase の二相で動く。
///
/// # Partial Result の例外
/// Operation Phase 中の Cancel またはエラーで途中終了した場合、書きかけの zip ファイルは
/// `ZipPathGuard` の Drop で自動削除する (壊れた `.zip` を残さない方針 - ADR-0001 Zip 例外)。
/// Scan Phase 中の Err は zip ファイル自体まだ作っていないので cleanup 不要。
///
/// # 旧 `fs::file::create_zip` からの挙動互換
/// - top-level `name` が既存なら `name_1`, `name_2`, ... に振り替える
/// - 再帰内の symlink エントリは zip に含めない (リンク先 follow による任意領域漏洩を回避)
/// - top-level の VFile が dir-symlink の場合は `metadata()` で follow しその内容を zip 化する
///   (`copy_to` と同じ「ユーザが明示的に指定した対象は follow」規約)
pub(super) fn run_zip_create(
    dir: &VFile,
    name: &str,
    files: &[VFile],
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<()> {
    // UI 層でも `anyhow::ensure!` で同等検証している前提だが、API 直接呼出に対する防御深度。
    anyhow::ensure!(
        !name.is_empty()
            && Path::new(name)
                .components()
                .all(|c| matches!(c, std::path::Component::Normal(_))),
        "{name}: Invalid zip name"
    );
    // Scan Phase: zip に詰めるエントリを集める (内部で cancel チェック)
    let Some(plan) = scan_zip_plan(files, cancel, on_progress)? else {
        return Ok(());
    };
    // 衝突回避: dir/name がすでに存在すれば name_1, name_2, ... に振り替える
    let zip_path = pick_unique_top_dest(
        &Path::new(dir.absolute_path()).join(name),
        &std::collections::HashSet::new(),
    )?;

    // Operation Phase: zip 書き出し。ファイル open に成功した直後から `ZipPathGuard` で
    // cleanup 範囲を限定する (open 失敗時に他プロセスのファイルを誤って削除しないため)。
    write_zip_plan(&plan, &zip_path, cancel, on_progress)?;
    Ok(())
}

/// 書きかけ zip を Drop 時に削除する RAII ガード。
/// Cancel / Err 経路で `let _ = remove_file()` を散らさず、cleanup を関数末尾で集約する。
/// `arm` で守備、通常完走時に `disarm` で解除。Drop 時の remove 失敗は `tracing::warn!` に残す
/// (`NotFound` は完走後に自分が削除済みのケースなので無視)。
struct ZipPathGuard<'a> {
    path: &'a Path,
    armed: bool,
}

impl<'a> ZipPathGuard<'a> {
    fn arm(path: &'a Path) -> Self {
        Self { path, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for ZipPathGuard<'_> {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        if let Err(e) = std::fs::remove_file(self.path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!("failed to remove partial zip {}: {e}", self.path.display());
        }
    }
}

/// Scan Phase の結果。
/// `directories`: zip 内に作成すべきディレクトリエントリ名 (例: `"foo/"`)。
/// `files`: zip に書き込むエントリ列 (src パス + zip 内パス)。
#[derive(Debug, Default)]
struct ZipPlan {
    directories: Vec<String>,
    files: Vec<ZipEntry>,
}

#[derive(Debug)]
struct ZipEntry {
    src: PathBuf,
    /// zip 内でのエントリ名 (forward slash 区切り)
    name: String,
}

fn scan_zip_plan(
    roots: &[VFile],
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<Option<ZipPlan>> {
    let mut plan = ZipPlan::default();
    on_progress(Phase::Scanning, 0, None);
    for root in roots {
        if cancel.load(Ordering::Relaxed) {
            return Ok(None);
        }
        let src = Path::new(root.absolute_path());
        let metadata = src
            .metadata()
            .with_context(|| format!("{}: Failed to stat source", src.display()))?;
        // zip 内エントリ名は src 親からの相対パス (= src の basename) を起点に組み立てる。
        let prefix = src.parent().unwrap_or(src);
        if metadata.is_dir() {
            match collect_zip_directory(src, prefix, &mut plan, cancel, on_progress)? {
                CollectStatus::Completed => {}
                CollectStatus::Cancelled => return Ok(None),
            }
        } else {
            let name = relative_zip_name(src, prefix)?;
            enqueue_zip_file(&mut plan, src.to_path_buf(), name, on_progress);
        }
    }
    Ok(Some(plan))
}

fn collect_zip_directory(
    src: &Path,
    prefix: &Path,
    plan: &mut ZipPlan,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<CollectStatus> {
    let dir_name = relative_zip_name(src, prefix)?;
    plan.directories.push(format!("{dir_name}/"));
    for entry in std::fs::read_dir(src)
        .with_context(|| format!("{}: Failed to read directory", src.display()))?
    {
        if cancel.load(Ordering::Relaxed) {
            return Ok(CollectStatus::Cancelled);
        }
        let entry =
            entry.with_context(|| format!("{}: Failed to read directory entry", src.display()))?;
        let entry_src = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("{}: Failed to read file type", entry_src.display()))?;
        // 既存 add_dir_to_zip と同じく symlink は zip に含めない (リンク先データの follow を避ける)
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            match collect_zip_directory(&entry_src, prefix, plan, cancel, on_progress)? {
                CollectStatus::Completed => {}
                CollectStatus::Cancelled => return Ok(CollectStatus::Cancelled),
            }
        } else {
            let name = relative_zip_name(&entry_src, prefix)?;
            enqueue_zip_file(plan, entry_src, name, on_progress);
        }
    }
    Ok(CollectStatus::Completed)
}

/// `prefix` からの相対パスを zip エントリ名 (`/` 区切り) に整形する。
/// `path` が `prefix` 配下にない場合や、エントリ名が空文字列になる場合は Err を返す
/// (`Component::Normal` 以外をフィルタする性質上、ルート path 等で空文字に縮退する経路を防ぐ)。
fn relative_zip_name(path: &Path, prefix: &Path) -> Result<String> {
    let relative = path.strip_prefix(prefix).with_context(|| {
        format!(
            "{} is not under zip prefix {}",
            path.display(),
            prefix.display()
        )
    })?;
    let name = relative
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/");
    anyhow::ensure!(
        !name.is_empty(),
        "{}: produced empty zip entry name",
        path.display()
    );
    Ok(name)
}

fn enqueue_zip_file(
    plan: &mut ZipPlan,
    src: PathBuf,
    name: String,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) {
    plan.files.push(ZipEntry { src, name });
    let count = plan.files.len();
    if count.is_multiple_of(SCAN_NOTIFY_BATCH) {
        on_progress(Phase::Scanning, count, None);
    }
}

fn write_zip_plan(
    plan: &ZipPlan,
    zip_path: &Path,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<CollectStatus> {
    // `OpenOptions::create_new(true)` で atomic に新規作成。open 失敗時は cleanup 対象外
    // (他プロセスのファイルを誤って消さないため、ガードはここから arm する)。
    let zip_file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(zip_path)
        .with_context(|| format!("{}: Failed to create zip file", zip_path.display()))?;
    let mut guard = ZipPathGuard::arm(zip_path);

    // central directory write の syscall 数を減らすため zip 出力側を BufWriter で包む
    let buffered = std::io::BufWriter::with_capacity(256 * 1024, zip_file);
    let mut writer = zip::ZipWriter::new(buffered);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let total = plan.files.len();
    on_progress(Phase::Zipping, 0, Some(total));

    if for_each_until_cancelled(&plan.directories, cancel, |dir_name| {
        writer.add_directory(dir_name, options).with_context(|| {
            format!(
                "{}: Failed to add directory {dir_name} to zip",
                zip_path.display()
            )
        })
    })?
    .is_cancelled()
    {
        return Ok(CollectStatus::Cancelled);
    }
    // 大量小ファイル時に read syscall 回数を削減するため BufReader で包む。
    // `std::io::copy` 失敗は I/O 由来の本物のエラー (例: disk full) として扱い `?` で伝播。
    if process_items(&plan.files, Phase::Zipping, cancel, on_progress, |entry| {
        writer.start_file(&entry.name, options).with_context(|| {
            format!(
                "{}: Failed to add {} to zip",
                zip_path.display(),
                entry.name
            )
        })?;
        let f = std::fs::File::open(&entry.src)
            .with_context(|| format!("{}: Failed to open source", entry.src.display()))?;
        let mut reader = std::io::BufReader::with_capacity(64 * 1024, f);
        std::io::copy(&mut reader, &mut writer)
            .with_context(|| format!("{}: Failed to write to zip", entry.src.display()))?;
        Ok(())
    })?
    .is_cancelled()
    {
        return Ok(CollectStatus::Cancelled);
    }
    // ZipWriter::finish() は内部の BufWriter を返す。BufWriter::into_inner() で flush を強制し、
    // 残ったバッファ未書出を Err に昇格させる (Drop 時の握り潰しを避ける)。
    let buffered = writer
        .finish()
        .with_context(|| format!("{}: Failed to finalize zip", zip_path.display()))?;
    buffered.into_inner().map_err(|e| {
        anyhow::anyhow!("{}: Failed to flush zip writer: {}", zip_path.display(), e)
    })?;
    guard.disarm();
    Ok(CollectStatus::Completed)
}

/// Move Job 本体。
/// 同一 FS では `rename` 一発で済むため Scan Phase を経由せず top-level 件数で進捗を出す。
/// `std::fs::rename` がクロスファイルシステムで失敗した場合は、全 root を Scan + Copy + Remove の
/// フォールバックパスに切り替える (UI 上は `Scanning... N files` → `Moving k/N files` の遷移)。
///
/// # Partial Result
/// Move における Partial Result は「src から消え、dest に存在する root」の集合と定義する。
/// 中途半端な状態 (src/dest 重複や、コピー途中) は **未完了 root** とみなし、
/// ユーザに伝わるべき "完了済み" には含めない。
///
/// 各タイミングごとの実ディスク状態:
/// - 同一 FS 高速パス中の cancel: 既に rename 済みの root は dest に、未処理 root は src に残る (= 完了 + 未完了)
/// - 同一 FS 高速パス中の rename Err (probe 以降): 既に rename 済みの root は dest に残る (エラー文言に明記)
/// - EXDEV フォールバック中の cancel:
///   - Scan 中: 何も変えない (完了 root 無し)
///   - Copy 中: dest にコピー済み + src に全 root → "完了" root はゼロ。Copy 完了せず src/dest 重複の生データが残る
///   - Remove 中: dest には全 root のコピーが揃い、src からは既に削除された root が消える (完了済み = src 消去された root)
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::async_job::FileJob;
    use crate::fs::async_job::test_support::{read_to_string, vfile, write_file};
    use std::fs::File;
    use tempfile::TempDir;

    fn read_zip_entries(zip_path: &std::path::Path) -> Vec<(String, Vec<u8>)> {
        let f = File::open(zip_path).unwrap();
        let mut archive = zip::ZipArchive::new(f).unwrap();
        let mut out = Vec::new();
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).unwrap();
            let name = entry.name().to_owned();
            if entry.is_dir() {
                out.push((name, Vec::new()));
            } else {
                let mut buf = Vec::new();
                std::io::copy(&mut entry, &mut buf).unwrap();
                out.push((name, buf));
            }
        }
        out
    }

    #[cfg(unix)]
    #[test]
    fn zip_create_skips_symlinks_inside_source_directory() {
        // src/foo/a.txt (通常ファイル) と src/foo/link -> ../outside (dir-symlink) を用意し、
        // symlink エントリが zip に含まれず outside の中身が漏れないことを検証する
        // (既存 add_dir_to_zip と同じ挙動)。
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("foo");
        write_file(&src_root.join("a.txt"), b"alpha");
        let outside = tmp.path().join("outside");
        write_file(&outside.join("secret.txt"), b"should-not-leak");
        std::os::unix::fs::symlink(&outside, src_root.join("link")).unwrap();

        let job = FileJob::ZipCreate {
            dir: vfile(tmp.path()),
            name: "out.zip".to_string(),
            files: vec![vfile(&src_root)],
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("ZipCreate should succeed");

        let entries = read_zip_entries(&tmp.path().join("out.zip"));
        let names: std::collections::HashSet<String> =
            entries.iter().map(|(n, _)| n.clone()).collect();
        assert!(names.contains("foo/a.txt"));
        // link 自体も outside/secret.txt も zip に含まれない
        assert!(
            !names
                .iter()
                .any(|n| n.contains("link") || n.contains("secret")),
            "symlink and its target must not appear in zip: {names:?}"
        );
    }

    #[test]
    fn zip_create_avoids_collision_by_appending_numeric_suffix() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("hello.txt");
        write_file(&src, b"new");
        // dir/out.zip がすでに存在 → unique_path で out_1.zip にずらす
        let existing_zip = tmp.path().join("out.zip");
        write_file(&existing_zip, b"existing");

        let job = FileJob::ZipCreate {
            dir: vfile(tmp.path()),
            name: "out.zip".to_string(),
            files: vec![vfile(&src)],
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("ZipCreate should succeed");

        // 既存ファイルは無傷 (zip でなくただのテキストファイルとして残る)
        assert_eq!(read_to_string(&existing_zip), "existing");
        // 新規 zip は out_1.zip に置かれる
        let new_zip = tmp.path().join("out_1.zip");
        assert!(
            new_zip.exists(),
            "new zip should be created at {}",
            new_zip.display()
        );
        let entries = read_zip_entries(&new_zip);
        assert_eq!(entries[0].0, "hello.txt");
        assert_eq!(entries[0].1, b"new");
    }

    #[test]
    fn zip_create_removes_partial_zip_when_cancelled_during_operation() {
        use std::sync::Arc;
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("a.txt");
        let b = tmp.path().join("b.txt");
        let c = tmp.path().join("c.txt");
        write_file(&a, b"a");
        write_file(&b, b"b");
        write_file(&c, b"c");

        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_for_closure = cancel.clone();
        let job = FileJob::ZipCreate {
            dir: vfile(tmp.path()),
            name: "out.zip".to_string(),
            files: vec![vfile(&a), vfile(&b), vfile(&c)],
        };
        // 1 ファイル zip 完了時に cancel をセット
        job.run(&cancel, &mut |p, n, _| {
            if p == Phase::Zipping && n == 1 {
                cancel_for_closure.store(true, Ordering::Relaxed);
            }
        })
        .expect("cancel should produce Ok early return");

        // 書きかけ zip は削除されている (Partial Result の Zip 例外)
        assert!(
            !tmp.path().join("out.zip").exists(),
            "incomplete zip should be removed on cancel"
        );
    }

    #[test]
    fn zip_create_emits_scanning_then_zipping_progress() {
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("a.txt");
        let b = tmp.path().join("b.txt");
        write_file(&a, b"a");
        write_file(&b, b"b");

        let mut events: Vec<(Phase, usize, Option<usize>)> = Vec::new();
        let job = FileJob::ZipCreate {
            dir: vfile(tmp.path()),
            name: "out.zip".to_string(),
            files: vec![vfile(&a), vfile(&b)],
        };
        job.run(&AtomicBool::new(false), &mut |p, n, t| {
            events.push((p, n, t))
        })
        .expect("ZipCreate should succeed");

        // Scanning 初期 0 通知のあと Zipping 0/2, 1/2, 2/2 と進む
        let scanning_first = events
            .iter()
            .find(|(p, _, _)| *p == Phase::Scanning)
            .copied();
        assert_eq!(scanning_first, Some((Phase::Scanning, 0, None)));
        let zipping: Vec<_> = events
            .iter()
            .filter(|(p, _, _)| *p == Phase::Zipping)
            .copied()
            .collect();
        assert_eq!(
            zipping,
            vec![
                (Phase::Zipping, 0, Some(2)),
                (Phase::Zipping, 1, Some(2)),
                (Phase::Zipping, 2, Some(2)),
            ]
        );
    }

    #[test]
    fn zip_create_recursively_zips_directory_contents() {
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("foo");
        write_file(&src_root.join("a.txt"), b"alpha");
        write_file(&src_root.join("bar").join("b.txt"), b"beta");

        let job = FileJob::ZipCreate {
            dir: vfile(tmp.path()),
            name: "tree.zip".to_string(),
            files: vec![vfile(&src_root)],
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("ZipCreate should succeed");

        let entries = read_zip_entries(&tmp.path().join("tree.zip"));
        let names: std::collections::HashSet<String> =
            entries.iter().map(|(n, _)| n.clone()).collect();
        assert!(
            names.contains("foo/a.txt"),
            "missing foo/a.txt in {names:?}"
        );
        assert!(
            names.contains("foo/bar/b.txt"),
            "missing foo/bar/b.txt in {names:?}"
        );
        let by_name: std::collections::HashMap<_, _> = entries.into_iter().collect();
        assert_eq!(
            by_name.get("foo/a.txt").map(|v| v.as_slice()),
            Some(b"alpha" as &[u8])
        );
        assert_eq!(
            by_name.get("foo/bar/b.txt").map(|v| v.as_slice()),
            Some(b"beta" as &[u8])
        );
    }

    #[test]
    fn zip_create_writes_multiple_files_into_archive() {
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("a.txt");
        let b = tmp.path().join("b.txt");
        write_file(&a, b"alpha");
        write_file(&b, b"beta");

        let job = FileJob::ZipCreate {
            dir: vfile(tmp.path()),
            name: "multi.zip".to_string(),
            files: vec![vfile(&a), vfile(&b)],
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("ZipCreate should succeed");

        let entries = read_zip_entries(&tmp.path().join("multi.zip"));
        let by_name: std::collections::HashMap<_, _> = entries.into_iter().collect();
        assert_eq!(
            by_name.get("a.txt").map(|v| v.as_slice()),
            Some(b"alpha" as &[u8])
        );
        assert_eq!(
            by_name.get("b.txt").map(|v| v.as_slice()),
            Some(b"beta" as &[u8])
        );
    }

    #[test]
    fn zip_create_rejects_name_with_path_traversal_components() {
        // 旧 fs::file::create_zip の Component::Normal 検証に相当 (Unzip 経路と対称形)。
        // `..` や絶対パスで dir の外に書き出すのを防ぐ防御深度のテスト。
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("hello.txt");
        write_file(&src, b"hello");

        for bad in ["", "../escape.zip", "sub/foo.zip"] {
            let job = FileJob::ZipCreate {
                dir: vfile(tmp.path()),
                name: bad.to_string(),
                files: vec![vfile(&src)],
            };
            let result = job.run(&AtomicBool::new(false), &mut |_, _, _| {});
            assert!(
                result.is_err(),
                "name {bad:?} should be rejected by Component::Normal validation"
            );
        }
    }

    #[test]
    fn zip_create_writes_single_file_into_archive() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("hello.txt");
        write_file(&src, b"hello fv");
        let dir = tmp.path().to_path_buf();

        let job = FileJob::ZipCreate {
            dir: vfile(&dir),
            name: "out.zip".to_string(),
            files: vec![vfile(&src)],
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("ZipCreate should succeed");

        let zip_path = dir.join("out.zip");
        assert!(
            zip_path.exists(),
            "zip file should be created at {}",
            zip_path.display()
        );
        let entries = read_zip_entries(&zip_path);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, "hello.txt");
        assert_eq!(entries[0].1, b"hello fv");
    }
}
