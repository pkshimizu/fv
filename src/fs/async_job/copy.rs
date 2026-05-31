//! Copy Job。Scan Phase で Copy Plan を組み立て、Operation Phase で 1 エントリずつコピーする。

use super::checkpoint::{for_each_until_cancelled, process_items};
use super::destination::{dest_is_exact_path, ensure_destination_dir};
use super::plan::{copy_entry, scan_copy_plan};
use crate::fs::VFile;
use crate::state::Phase;
use anyhow::{Context, Result};
use std::path::Path;
use std::sync::atomic::AtomicBool;

/// Copy Job 本体。Scan Phase → Operation Phase の二相で動く。
///
/// # Partial Result
/// Cancel された場合、Operation Phase で `std::fs::copy` 完了済みの個別ファイルは
/// ディスクに残る。それを内包する祖先ディレクトリも残る (空ディレクトリとして残り得る)。
/// Scan Phase 中の cancel では Partial Result は残らない (mkdir も発火していないため)。
///
/// # Symlink
/// top-level の VFile が dir-symlink の場合はリンクをたどってその内容をコピーする
/// (旧 `fs::file::copy_to` と同じ挙動)。再帰内ではリンクをたどらず、symlink エントリは
/// `std::fs::copy` で「リンク先データを書き出すファイル」として扱う。これにより
/// 入れ子の symlink ループや任意領域への脱出を防ぐ。
pub(super) fn run_copy(
    files: &[VFile],
    dest: &Path,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<()> {
    // dest の解釈（正確な宛先パス or コンテナ）は dest.is_dir() の stat を含むため
    // 一度だけ評価し、Scan Phase と dir 確保の双方で同じ結果を使う。
    let exact_path = dest_is_exact_path(files, dest);
    let Some(plan) = scan_copy_plan(files, dest, exact_path, cancel, on_progress)? else {
        // Scan Phase 中に cancel された場合は Partial Result なしで早期 return
        return Ok(());
    };
    let total = plan.files.len();
    // Operation Phase 開始を Phase 切り替え直後の `(Copying, 0, Some(total))` で通知。
    // mkdir ループに入る前に出すことで「ディレクトリ作成中は UI が Scanning のまま」を回避する。
    on_progress(Phase::Copying, 0, Some(total));
    // ユーザ指定の宛先に応じてコンテナ dir（または宛先パスの親）を一度だけ確保。
    // それ以降の plan.directories は pre-order により親が常に作成済みなので create_dir で十分。
    ensure_destination_dir(exact_path, dest)?;
    if for_each_until_cancelled(&plan.directories, cancel, |dir| {
        std::fs::create_dir(dir)
            .with_context(|| format!("{}: Failed to create directory", dir.display()))
    })?
    .is_cancelled()
    {
        return Ok(());
    }
    // 末尾の files コピーは完走/中断どちらでも後続がないため、戻り値の CollectStatus は捨てる。
    process_items(&plan.files, Phase::Copying, cancel, on_progress, copy_entry)?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::async_job::FileJob;
    use crate::fs::async_job::test_support::{read_to_string, vfile, write_file};
    use std::sync::atomic::Ordering;
    use tempfile::TempDir;

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
        assert_eq!(read_to_string(&dest.join("foo_1").join("a.txt")), "alpha");
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
        job.run(&AtomicBool::new(true), &mut |p, n, t| {
            events.push((p, n, t))
        })
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
        job.run(&AtomicBool::new(false), &mut |p, n, t| {
            events.push((p, n, t))
        })
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
        job.run(&AtomicBool::new(false), &mut |p, n, t| {
            events.push((p, n, t))
        })
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

    #[test]
    fn copy_single_file_to_nonexistent_path_writes_that_exact_file() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("file.txt");
        write_file(&src, b"hello fv");
        // 既存しない宛先パス（ファイル名）を指定する。
        let dest = tmp.path().join("file_2.txt");

        let job = FileJob::Copy {
            files: vec![vfile(&src)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("Copy should succeed");

        // dest はちょうどそのパスのファイルになる（ディレクトリ化しない）。
        assert!(dest.is_file(), "dest should be a file, not a directory");
        assert_eq!(read_to_string(&dest), "hello fv");
    }

    #[test]
    fn copy_multiple_files_to_nonexistent_dest_creates_container_directory() {
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("a.txt");
        let b = tmp.path().join("b.txt");
        write_file(&a, b"alpha");
        write_file(&b, b"bravo");
        // 既存しない宛先。複数ソースなのでコンテナディレクトリとして作成されるべき。
        let dest = tmp.path().join("box");

        let job = FileJob::Copy {
            files: vec![vfile(&a), vfile(&b)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("Copy should succeed");

        assert!(dest.is_dir(), "dest should be created as a directory");
        assert_eq!(read_to_string(&dest.join("a.txt")), "alpha");
        assert_eq!(read_to_string(&dest.join("b.txt")), "bravo");
    }

    #[test]
    fn copy_single_file_to_existing_file_path_appends_numeric_suffix() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("a.txt");
        write_file(&src, b"alpha");
        // 明示した宛先パスが既存ファイルのケース。上書きせず _1 を付与する。
        let dest = tmp.path().join("b.txt");
        write_file(&dest, b"existing");

        let job = FileJob::Copy {
            files: vec![vfile(&src)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("Copy should succeed");

        // 既存ファイルは無傷、コピーは b_1.txt に置かれる。
        assert_eq!(read_to_string(&dest), "existing");
        assert_eq!(read_to_string(&tmp.path().join("b_1.txt")), "alpha");
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
        write_file(
            &outside.join("secret.txt"),
            b"should-not-be-recursively-copied",
        );
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
            !dest
                .join("src")
                .join("loop")
                .join("loop")
                .join("loop")
                .exists()
                || loop_meta.file_type().is_symlink(),
            "self-loop should not produce nested loop directories"
        );
    }
}
