//! Delete Job。Scan Phase で削除規模を再帰列挙し、Operation Phase で top-level を順次 trash へ送る。

use super::checkpoint::{CollectStatus, SCAN_NOTIFY_BATCH, notify_scan_progress, process_items};
use crate::fs::VFile;
use crate::state::Phase;
use anyhow::{Context, Result};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// Delete Job 本体。Scan Phase で削除対象を再帰列挙して件数をユーザに見せ、
/// Operation Phase では top-level の VFile を順次 `trash::delete` で削除する。
///
/// # Partial Result
/// Operation Phase 中の cancel: 既に `trash::delete` 済みの root は trash 側に残り、
/// 未着手の root は元の場所に残る。trash::delete は atomic per-item なので、
/// 半端に削除されたディレクトリ等の grey state は発生しない。
///
/// # 進捗 N の意味
/// - Scanning: 再帰ファイル数 (情報提示)
/// - Deleting: top-level VFile 件数 (`trash::delete` の単位)
///
/// 両 phase で N の意味が異なる点に注意 (trash::delete が atomic per-item のため)。
pub(super) fn run_delete(
    files: &[VFile],
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<()> {
    run_delete_with(files, cancel, on_progress, &mut delete_path)
}

/// 本番経路で使う削除。通常ファイル/ディレクトリはシステムのゴミ箱へ送る。
/// シンボリックリンクは macOS の `trashItemAtURL` がリンクをゴミ箱へ送れず権限エラーに
/// なるため、リンク自体を直接削除する（`remove_file` はリンクをたどらず本体のみ消す）。
/// `path` を含む context はここで終わらせて、上位 (`run_delete_with`) では「delete 操作の
/// 責務」レベル (進捗 hint) のみ被せる役割分担にする。
fn delete_path(path: &Path) -> Result<()> {
    let is_symlink = path
        .symlink_metadata()
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false);
    if is_symlink {
        std::fs::remove_file(path)
            .with_context(|| format!("{}: Failed to remove symlink", path.display()))
    } else {
        trash::delete(path).with_context(|| format!("{}: Failed to move to trash", path.display()))
    }
}

/// `delete_fn` 注入版。テストでは `std::fs::remove_*` 系を渡して
/// 実 trash 経由を避ける (ユーザのシステム trash を汚さない)。
fn run_delete_with(
    files: &[VFile],
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
    delete_fn: &mut dyn FnMut(&Path) -> Result<()>,
) -> Result<()> {
    if files.is_empty() {
        return Ok(());
    }
    // Scan Phase: 再帰列挙で削除対象のファイル数を数え、ユーザに削除規模を提示する。
    // Operation Phase は top-level VFile を 1 単位として `delete_fn` を呼ぶので、
    // Scan の総数とは N の意味が変わる点に注意 (run_delete doc 参照)。
    if scan_delete_count(files, cancel, on_progress)?.is_cancelled() {
        return Ok(());
    }
    on_progress(Phase::Deleting, 0, Some(files.len()));
    // 残件数は Progress (processed/total) で別途 UI に届くため、エラー context には載せない。
    process_items(files, Phase::Deleting, cancel, on_progress, |file| {
        delete_fn(Path::new(file.absolute_path())).context("Delete aborted")
    })?;
    Ok(())
}

/// 削除対象のファイル数を再帰的にカウントし、`(Scanning, k, None)` を batch で発火する。
/// トップレベル symlink はカウント 1 として扱う (Operation Phase でリンク自体を削除する)。
///
/// `metadata()` (symlink follow) ではなく `symlink_metadata()` を使う。`metadata()` だと
/// 例えば `~/Documents -> /` のような dir-symlink が選ばれていた場合、follow 先の巨大ツリーを
/// 不意に再帰列挙してしまうため。Operation Phase は `trash::delete` 1 回しか呼ばないので
/// Scan で follow する利益はない。
fn scan_delete_count(
    roots: &[VFile],
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<CollectStatus> {
    on_progress(Phase::Scanning, 0, None);
    let mut count = 0usize;
    for root in roots {
        if cancel.load(Ordering::Relaxed) {
            return Ok(CollectStatus::Cancelled);
        }
        let src = Path::new(root.absolute_path());
        // stat 失敗を「ファイル扱い」に握りつぶさず Scan で早期 Err として顕在化する。
        let metadata = src
            .symlink_metadata()
            .with_context(|| format!("{}: Failed to stat source", src.display()))?;
        let file_type = metadata.file_type();
        if file_type.is_dir() && !file_type.is_symlink() {
            match walk_count_for_delete(src, &mut count, cancel, on_progress)? {
                CollectStatus::Completed => {}
                CollectStatus::Cancelled => return Ok(CollectStatus::Cancelled),
            }
        } else {
            count += 1;
            notify_scan_progress(count, on_progress);
        }
    }
    // Scan Phase の最後で端数件数を通知 (バッチ境界で終わらない場合の取りこぼし対策)。
    // ちょうど SCAN_NOTIFY_BATCH の倍数で終わったときは notify_scan_progress で発火済みなのでスキップ。
    // count == 0 のときは関数冒頭の入口 emit (Scanning, 0, None) と同値になるのでスキップ。
    if count > 0 && !count.is_multiple_of(SCAN_NOTIFY_BATCH) {
        on_progress(Phase::Scanning, count, None);
    }
    Ok(CollectStatus::Completed)
}

/// `src` ディレクトリ配下を再帰的に列挙し、ファイル数を `count` に加算する。
///
/// - 再帰中の symlink は follow せず、`count += 1` で 1 エントリとして数える
///   (`trash::delete` は top-level しか触らないため、内部 symlink の follow は無意味かつ
///   任意領域脱出リスクになりうる)
/// - `read_dir` / `DirEntry::file_type` の I/O Err は `with_context` で対象パスを含めて伝播する
/// - 各エントリ処理前に cancel をチェックする File-level Checkpoint
fn walk_count_for_delete(
    src: &Path,
    count: &mut usize,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<CollectStatus> {
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
        if file_type.is_dir() && !file_type.is_symlink() {
            match walk_count_for_delete(&entry_src, count, cancel, on_progress)? {
                CollectStatus::Completed => {}
                CollectStatus::Cancelled => return Ok(CollectStatus::Cancelled),
            }
        } else {
            *count += 1;
            notify_scan_progress(*count, on_progress);
        }
    }
    Ok(CollectStatus::Completed)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::async_job::test_support::{vfile, write_file};
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[cfg(unix)]
    #[test]
    fn delete_path_removes_symlink_and_keeps_target() {
        // symlink は trash::delete が macOS で権限エラーになるため、リンク自体を
        // remove_file で直接削除する。リンクはたどらず、ターゲットは残す。
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("real.txt");
        std::fs::write(&target, "data").unwrap();
        let link = tmp.path().join("link.txt");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        delete_path(&link).unwrap();

        assert!(link.symlink_metadata().is_err(), "リンクは削除される");
        assert!(target.exists(), "リンク先（ターゲット）は残る");
    }

    #[test]
    fn delete_returns_err_when_source_is_missing() {
        // Scan Phase の metadata() で stat 失敗 → Err 早期返却 (ファイル扱いに握りつぶさない)
        let tmp = TempDir::new().unwrap();
        let mut delete_fn =
            |_: &Path| -> Result<()> { panic!("delete_fn must not be called when scan fails") };
        let result = run_delete_with(
            &[vfile(&tmp.path().join("no-such.txt"))],
            &AtomicBool::new(false),
            &mut |_, _, _| {},
            &mut delete_fn,
        );
        assert!(result.is_err());
    }

    #[test]
    fn delete_aborts_on_first_error_with_file_path_in_context() {
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("a.txt");
        let b = tmp.path().join("b.txt");
        write_file(&a, b"a");
        write_file(&b, b"b");

        let mut delete_fn = |path: &Path| -> Result<()> {
            if path.ends_with("b.txt") {
                anyhow::bail!("simulated permission denied")
            }
            std::fs::remove_file(path)?;
            Ok(())
        };
        let result = run_delete_with(
            &[vfile(&a), vfile(&b)],
            &AtomicBool::new(false),
            &mut |_, _, _| {},
            &mut delete_fn,
        );
        let err = result.expect_err("Err should propagate on delete_fn failure");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("Delete aborted"),
            "Err context should mark the delete as aborted: {msg}"
        );
        assert!(
            msg.contains("simulated permission denied"),
            "Err chain should preserve the original cause: {msg}"
        );
        // 1 件目は完了、2 件目で abort
        assert!(!a.exists(), "first file deleted before error");
        assert!(b.exists(), "second file remains (delete_fn failed)");
    }

    #[test]
    fn delete_keeps_partial_result_when_cancelled_during_operation() {
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
        let mut delete_fn = |path: &Path| -> Result<()> {
            std::fs::remove_file(path)?;
            Ok(())
        };
        run_delete_with(
            &[vfile(&a), vfile(&b), vfile(&c)],
            &cancel,
            &mut |p, n, _| {
                if p == Phase::Deleting && n == 1 {
                    cancel_for_closure.store(true, Ordering::Relaxed);
                }
            },
            &mut delete_fn,
        )
        .expect("cancel should produce Ok early return");

        // 1 件削除済み、残り 2 件は元の場所に残る (Partial Result on filesystem)
        assert!(!a.exists(), "first file should be deleted");
        assert!(
            b.exists(),
            "second file should remain (cancel before delete)"
        );
        assert!(
            c.exists(),
            "third file should remain (cancel before delete)"
        );
    }

    #[test]
    fn delete_emits_scanning_then_deleting_progress() {
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("a.txt");
        let b = tmp.path().join("b.txt");
        write_file(&a, b"a");
        write_file(&b, b"b");

        let mut events: Vec<(Phase, usize, Option<usize>)> = Vec::new();
        let mut delete_fn = |path: &Path| -> Result<()> {
            std::fs::remove_file(path)?;
            Ok(())
        };
        run_delete_with(
            &[vfile(&a), vfile(&b)],
            &AtomicBool::new(false),
            &mut |p, n, t| events.push((p, n, t)),
            &mut delete_fn,
        )
        .expect("Delete should succeed");

        // 初期 Scanning emit のあと Deleting 0..N のシーケンス
        let scanning_first = events
            .iter()
            .find(|(p, _, _)| *p == Phase::Scanning)
            .copied();
        assert_eq!(scanning_first, Some((Phase::Scanning, 0, None)));
        let deleting: Vec<_> = events
            .iter()
            .filter(|(p, _, _)| *p == Phase::Deleting)
            .copied()
            .collect();
        assert_eq!(
            deleting,
            vec![
                (Phase::Deleting, 0, Some(2)),
                (Phase::Deleting, 1, Some(2)),
                (Phase::Deleting, 2, Some(2)),
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn delete_does_not_recurse_into_top_level_dir_symlink() {
        // top-level が dir-symlink の場合、Scan Phase で follow して中身を再帰列挙しない。
        // (リンク先の任意領域の Scan による暴走防止)
        use std::os::unix::fs::symlink;
        let tmp = TempDir::new().unwrap();
        let real_dir = tmp.path().join("real");
        std::fs::create_dir(&real_dir).unwrap();
        write_file(&real_dir.join("inside.txt"), b"x");
        let link = tmp.path().join("link");
        symlink(&real_dir, &link).unwrap();

        let mut max_scan: usize = 0;
        let mut delete_fn = |_path: &Path| -> Result<()> {
            // symlink の trash::delete 相当: リンクのみ削除 (実体は触らない)
            std::fs::remove_file(_path)?;
            Ok(())
        };
        run_delete_with(
            &[vfile(&link)],
            &AtomicBool::new(false),
            &mut |p, n, _| {
                if p == Phase::Scanning {
                    max_scan = max_scan.max(n);
                }
            },
            &mut delete_fn,
        )
        .expect("Delete should succeed");

        // Scan は symlink を 1 件として数えるだけ
        assert_eq!(max_scan, 1);
        // リンクは消え、リンク先の中身は無傷
        assert!(!link.exists());
        assert!(real_dir.join("inside.txt").exists());
    }

    #[test]
    fn delete_treats_directory_as_single_top_level_item() {
        // trash::delete は atomic per-item で dir を 1 エントリとして trash に入れるため、
        // Operation Phase でも再帰展開せず top-level の dir 1 回の delete_fn 呼出にとどめる。
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("foo");
        write_file(&src_root.join("a.txt"), b"alpha");
        write_file(&src_root.join("bar").join("b.txt"), b"beta");

        let mut called: Vec<PathBuf> = Vec::new();
        let mut delete_fn = |path: &Path| -> Result<()> {
            called.push(path.to_path_buf());
            std::fs::remove_dir_all(path)?;
            Ok(())
        };
        run_delete_with(
            &[vfile(&src_root)],
            &AtomicBool::new(false),
            &mut |_, _, _| {},
            &mut delete_fn,
        )
        .expect("Delete should succeed");

        assert_eq!(called, vec![src_root.clone()]);
        assert!(!src_root.exists());
    }

    #[test]
    fn delete_invokes_delete_fn_for_all_top_level_files() {
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("a.txt");
        let b = tmp.path().join("b.txt");
        let c = tmp.path().join("c.txt");
        write_file(&a, b"a");
        write_file(&b, b"b");
        write_file(&c, b"c");

        let mut called: Vec<PathBuf> = Vec::new();
        let mut delete_fn = |path: &Path| -> Result<()> {
            called.push(path.to_path_buf());
            std::fs::remove_file(path)?;
            Ok(())
        };
        run_delete_with(
            &[vfile(&a), vfile(&b), vfile(&c)],
            &AtomicBool::new(false),
            &mut |_, _, _| {},
            &mut delete_fn,
        )
        .expect("Delete should succeed");

        assert_eq!(called, vec![a.clone(), b.clone(), c.clone()]);
        assert!(!a.exists() && !b.exists() && !c.exists());
    }
}
