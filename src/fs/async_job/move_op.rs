//! Move Job。同一FS は rename 高速パス、クロスFS は Copy Plan を使う copy + remove フォールバック。

use super::checkpoint::{for_each_until_cancelled, process_items};
use super::destination::{
    TopLevelPair, dest_is_exact_path, ensure_destination_dir, resolve_top_level_pairs,
};
use super::plan::{copy_entry, scan_pairs_into_plan};
use crate::fs::VFile;
use crate::state::Phase;
use anyhow::{Context, Result};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// `std::fs::rename` の戻り Err がクロスファイルシステムを示すか判定する。
/// `std::io::ErrorKind::CrossesDevices` で表現できる場合はそれを優先し、Unix 上の古い API でも
/// `libc::EXDEV` (`raw_os_error`) で fallback する。
fn is_cross_device_error(e: &std::io::Error) -> bool {
    if e.kind() == std::io::ErrorKind::CrossesDevices {
        return true;
    }
    #[cfg(unix)]
    {
        e.raw_os_error() == Some(libc::EXDEV)
    }
    #[cfg(not(unix))]
    {
        false
    }
}

pub(super) fn run_move(
    files: &[VFile],
    dest: &Path,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<()> {
    if files.is_empty() {
        return Ok(());
    }
    if cancel.load(Ordering::Relaxed) {
        return Ok(());
    }
    // dest の解釈は一度だけ評価して ensure と resolve の双方で同じ結果を使う。
    let exact_path = dest_is_exact_path(files, dest);
    // `run_copy` と同じく、入口でコンテナ dir（または宛先パスの親）を確保する
    // (rename も create_dir_all 同等の効果は無いため)。
    ensure_destination_dir(exact_path, dest)?;
    let pairs = resolve_top_level_pairs(files, dest, exact_path)?;

    // 副作用つき probe: 先頭 root への rename を 1 回だけ試し、成功なら同一 FS 高速パスに乗り、
    // CrossesDevices ならフォールバックパスへ。先頭 root はこの時点で本処理を 1 件分消費しているため、
    // 後続ループは `skip(1)` する。
    let first = &pairs[0];
    match std::fs::rename(&first.src, &first.dst) {
        Ok(()) => {
            let total = pairs.len();
            on_progress(Phase::Moving, 0, Some(total));
            on_progress(Phase::Moving, 1, Some(total));
            for (i, pair) in pairs.iter().enumerate().skip(1) {
                if cancel.load(Ordering::Relaxed) {
                    return Ok(());
                }
                std::fs::rename(&pair.src, &pair.dst).with_context(|| {
                    format!(
                        "{} -> {}: Failed to rename (other roots may have been moved already)",
                        pair.src.display(),
                        pair.dst.display()
                    )
                })?;
                on_progress(Phase::Moving, i + 1, Some(total));
            }
            Ok(())
        }
        Err(e) if is_cross_device_error(&e) => {
            move_via_copy_and_remove(&pairs, cancel, on_progress)
        }
        Err(e) => Err(anyhow::Error::from(e).context(format!(
            "{} -> {}: Failed to rename",
            first.src.display(),
            first.dst.display()
        ))),
    }
}

/// EXDEV フォールバック: 事前解決済みペア列を Scan + Copy + Remove で移動する。
/// 進捗 phase は Scan 中 `Scanning`、Copy 以降 `Moving` (Remove ステップ中は最終 `(Moving, N, Some(N))` を保持)。
fn move_via_copy_and_remove(
    pairs: &[TopLevelPair],
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) -> Result<()> {
    let Some(plan) = scan_pairs_into_plan(pairs, cancel, on_progress)? else {
        // Scan 中の cancel は Partial Result なしで早期 return
        return Ok(());
    };
    let total = plan.files.len();
    on_progress(Phase::Moving, 0, Some(total));
    // `run_copy` は冒頭で `ensure_dest_dir` するが、`run_move` の入口でも既に行っているため
    // ここでは plan.directories のみを `create_dir` で順次作成する。
    if for_each_until_cancelled(&plan.directories, cancel, |dir| {
        std::fs::create_dir(dir)
            .with_context(|| format!("{}: Failed to create directory", dir.display()))
    })?
    .is_cancelled()
    {
        return Ok(());
    }
    if process_items(&plan.files, Phase::Moving, cancel, on_progress, copy_entry)?.is_cancelled() {
        return Ok(());
    }
    // 全 Copy 完了後に各 root の src を削除する。
    // この最終ループは末尾なので完走/中断どちらでも後続はなく、戻り値の CollectStatus は捨てる。
    // `Path::is_dir()` は symlink を follow するため、dir-symlink を root に持つケースでリンク先の
    // ディレクトリを誤って `remove_dir_all` 経由で削除しに行ってしまう (現代の `remove_dir_all` は
    // `O_NOFOLLOW` で防御するが、防御深度として呼び出し側でも symlink_metadata で判定を分離する)。
    for_each_until_cancelled(pairs, cancel, |pair| {
        let meta = std::fs::symlink_metadata(&pair.src).with_context(|| {
            format!("{}: Failed to stat source for removal", pair.src.display())
        })?;
        let file_type = meta.file_type();
        let result = if file_type.is_symlink() || !file_type.is_dir() {
            std::fs::remove_file(&pair.src)
        } else {
            std::fs::remove_dir_all(&pair.src)
        };
        result.with_context(|| {
            format!(
                "{}: Failed to remove move source (destination already populated)",
                pair.src.display()
            )
        })
    })?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::async_job::FileJob;
    use crate::fs::async_job::test_support::{read_to_string, vfile, write_file};
    use tempfile::TempDir;

    #[test]
    fn move_single_file_to_nonexistent_path_writes_that_exact_file() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("file.txt");
        write_file(&src, b"hello fv");
        let dest = tmp.path().join("file_2.txt");

        let job = FileJob::Move {
            files: vec![vfile(&src)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("Move should succeed");

        // dest はそのパスのファイル、ソースは消える。
        assert!(dest.is_file(), "dest should be a file, not a directory");
        assert_eq!(read_to_string(&dest), "hello fv");
        assert!(!src.exists(), "source should be removed after move");
    }

    #[test]
    fn move_returns_err_when_source_is_missing() {
        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let job = FileJob::Move {
            files: vec![vfile(&tmp.path().join("no-such.txt"))],
            dest,
        };
        let result = job.run(&AtomicBool::new(false), &mut |_, _, _| {});
        assert!(result.is_err());
    }

    #[test]
    fn move_via_copy_and_remove_completes_scan_copy_remove_sequence() {
        // EXDEV フォールバック関数を直接呼び出して動作検証する (実際の cross-FS は CI で再現困難)。
        // 結果として src は消え dest にすべてのファイルが入り、進捗は Scanning → Moving 順で発火する。
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("foo");
        write_file(&src_root.join("a.txt"), b"alpha");
        write_file(&src_root.join("bar").join("b.txt"), b"beta");
        let dest_root = tmp.path().join("out").join("foo");
        std::fs::create_dir_all(dest_root.parent().unwrap()).unwrap();

        let pairs = vec![TopLevelPair {
            src: src_root.clone(),
            dst: dest_root.clone(),
        }];
        let mut events: Vec<(Phase, usize, Option<usize>)> = Vec::new();
        move_via_copy_and_remove(&pairs, &AtomicBool::new(false), &mut |p, n, t| {
            events.push((p, n, t))
        })
        .expect("EXDEV fallback should succeed");

        // dest にファイル群がコピーされている
        assert_eq!(read_to_string(&dest_root.join("a.txt")), "alpha");
        assert_eq!(read_to_string(&dest_root.join("bar").join("b.txt")), "beta");
        // src は削除されている
        assert!(
            !src_root.exists(),
            "src must be removed after move fallback"
        );

        // 進捗: Scanning 始まり → Moving に遷移
        assert!(
            events.iter().any(|(p, _, _)| *p == Phase::Scanning),
            "Scanning phase should be emitted: {events:?}"
        );
        let moving_events: Vec<_> = events
            .iter()
            .filter(|(p, _, _)| *p == Phase::Moving)
            .copied()
            .collect();
        // 初期 (Moving, 0, Some(2)) と各ファイルコピー後の通知 = (Moving, 2, Some(2)) で終わる
        let last = moving_events.last().expect("Moving emit must exist");
        assert_eq!(*last, (Phase::Moving, 2, Some(2)));
    }

    #[test]
    fn move_via_copy_and_remove_keeps_partial_result_when_cancelled_during_copy() {
        // Copy 中の cancel で src は残り、dest には部分結果が積まれていることを確認する。
        use std::sync::Arc;
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("foo");
        write_file(&src_root.join("a.txt"), b"a");
        write_file(&src_root.join("b.txt"), b"b");
        write_file(&src_root.join("c.txt"), b"c");
        let dest_root = tmp.path().join("out").join("foo");
        std::fs::create_dir_all(dest_root.parent().unwrap()).unwrap();

        let pairs = vec![TopLevelPair {
            src: src_root.clone(),
            dst: dest_root.clone(),
        }];
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_for_closure = cancel.clone();
        move_via_copy_and_remove(&pairs, &cancel, &mut |p, n, _| {
            // 1 ファイルコピー完了直後に cancel をセット
            if p == Phase::Moving && n == 1 {
                cancel_for_closure.store(true, Ordering::Relaxed);
            }
        })
        .expect("cancel should produce Ok early return");

        // src は手付かずで残る (Partial Result on src)
        assert!(src_root.exists(), "src must remain after Copy-time cancel");
        // dest にはコピー済みファイルが残る (Partial Result on dest)
        let copied_count = std::fs::read_dir(&dest_root)
            .unwrap()
            .filter_map(|e| e.ok())
            .count();
        assert_eq!(
            copied_count, 1,
            "exactly one file should be copied as partial result, got {copied_count}"
        );
    }

    #[test]
    fn move_avoids_collision_among_multiple_same_name_roots() {
        // a/foo.txt と b/foo.txt を同じ dest に Move する。
        // 同一 batch 内の衝突を `claimed` set で避け、両方を独立に dest 配下に置く。
        let tmp = TempDir::new().unwrap();
        let src_a = tmp.path().join("a").join("foo.txt");
        let src_b = tmp.path().join("b").join("foo.txt");
        write_file(&src_a, b"AAA");
        write_file(&src_b, b"BBB");
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let job = FileJob::Move {
            files: vec![vfile(&src_a), vfile(&src_b)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("Move should succeed");

        // 両 src が移動済み
        assert!(!src_a.exists());
        assert!(!src_b.exists());
        // 両方の内容が dest 配下に独立して残っている (foo.txt と foo_1.txt)
        let entries: Vec<_> = std::fs::read_dir(&dest)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name())
            .collect();
        assert_eq!(
            entries.len(),
            2,
            "both files should be moved with unique names, got {entries:?}"
        );
        let names: std::collections::HashSet<String> = entries
            .iter()
            .map(|n| n.to_string_lossy().into_owned())
            .collect();
        assert!(names.contains("foo.txt"));
        assert!(names.contains("foo_1.txt"));
    }

    #[cfg(unix)]
    #[test]
    fn move_via_copy_and_remove_handles_top_level_dir_symlink_safely() {
        // src/link -> real/ の dir-symlink を top-level に渡し、EXDEV フォールバックで
        // src 側削除時に `remove_dir_all` がリンク先 real/ を消さないことを検証する。
        let tmp = TempDir::new().unwrap();
        let real_dir = tmp.path().join("real");
        write_file(&real_dir.join("inside.txt"), b"content");
        let symlink_root = tmp.path().join("link");
        std::os::unix::fs::symlink(&real_dir, &symlink_root).unwrap();
        let dest_root = tmp.path().join("out").join("link");
        std::fs::create_dir_all(dest_root.parent().unwrap()).unwrap();

        let pairs = vec![TopLevelPair {
            src: symlink_root.clone(),
            dst: dest_root.clone(),
        }];
        move_via_copy_and_remove(&pairs, &AtomicBool::new(false), &mut |_, _, _| {})
            .expect("EXDEV fallback should handle dir-symlink at top level");

        // symlink のリンク自体は削除されている
        assert!(
            std::fs::symlink_metadata(&symlink_root).is_err(),
            "top-level dir-symlink should be removed"
        );
        // リンク先の real ディレクトリは無傷
        assert!(
            real_dir.is_dir(),
            "linked target directory must NOT be removed"
        );
        assert_eq!(read_to_string(&real_dir.join("inside.txt")), "content");
        // dest 側にはリンク先の内容がコピーされている
        assert_eq!(read_to_string(&dest_root.join("inside.txt")), "content");
    }

    #[test]
    fn move_avoids_collision_by_appending_numeric_suffix() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("foo.txt");
        write_file(&src, b"new");
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();
        // dest/foo.txt がすでに存在 → unique_path で foo_1.txt にずらす
        write_file(&dest.join("foo.txt"), b"existing");

        let job = FileJob::Move {
            files: vec![vfile(&src)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("Move should succeed");

        // 既存ファイルは無傷
        assert_eq!(read_to_string(&dest.join("foo.txt")), "existing");
        // 移動は foo_1.txt に
        assert_eq!(read_to_string(&dest.join("foo_1.txt")), "new");
        assert!(!src.exists());
    }

    #[test]
    fn move_stops_when_cancel_is_preset() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("hello.txt");
        write_file(&src, b"hello");
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let job = FileJob::Move {
            files: vec![vfile(&src)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(true), &mut |_, _, _| {})
            .expect("cancel should produce Ok early return");

        // 事前 cancel なので rename は発火していない
        assert!(src.exists(), "src file should remain untouched");
        assert!(!dest.join("hello.txt").exists());
    }

    #[test]
    fn move_emits_top_level_progress_on_same_filesystem_path() {
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("a.txt");
        let b = tmp.path().join("b.txt");
        let c = tmp.path().join("c.txt");
        write_file(&a, b"a");
        write_file(&b, b"b");
        write_file(&c, b"c");
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let mut events: Vec<(Phase, usize, Option<usize>)> = Vec::new();
        let job = FileJob::Move {
            files: vec![vfile(&a), vfile(&b), vfile(&c)],
            dest,
        };
        job.run(&AtomicBool::new(false), &mut |p, n, t| {
            events.push((p, n, t))
        })
        .expect("Move should succeed");

        // 同一 FS パスでは Scan Phase をスキップし、Moving の top-level 件数のみ通知
        let moving: Vec<_> = events
            .iter()
            .filter(|(p, _, _)| *p == Phase::Moving)
            .copied()
            .collect();
        assert_eq!(
            moving,
            vec![
                (Phase::Moving, 0, Some(3)),
                (Phase::Moving, 1, Some(3)),
                (Phase::Moving, 2, Some(3)),
                (Phase::Moving, 3, Some(3)),
            ]
        );
        // Scan Phase は emit されない
        assert!(
            !events.iter().any(|(p, _, _)| *p == Phase::Scanning),
            "same-FS move should skip Scan Phase: {events:?}"
        );
    }

    #[test]
    fn move_renames_directory_atomically_on_same_filesystem() {
        let tmp = TempDir::new().unwrap();
        let src_root = tmp.path().join("foo");
        write_file(&src_root.join("a.txt"), b"alpha");
        write_file(&src_root.join("bar").join("b.txt"), b"beta");
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let job = FileJob::Move {
            files: vec![vfile(&src_root)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("Move should succeed");

        // dest 配下に階層が再現されている
        assert_eq!(read_to_string(&dest.join("foo").join("a.txt")), "alpha");
        assert_eq!(
            read_to_string(&dest.join("foo").join("bar").join("b.txt")),
            "beta"
        );
        // src からはディレクトリごと消えている
        assert!(
            !src_root.exists(),
            "src directory should be gone after move"
        );
    }

    #[test]
    fn move_renames_single_file_to_destination_directory_on_same_filesystem() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("hello.txt");
        write_file(&src, b"hello fv");
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();

        let job = FileJob::Move {
            files: vec![vfile(&src)],
            dest: dest.clone(),
        };
        job.run(&AtomicBool::new(false), &mut |_, _, _| {})
            .expect("Move should succeed");

        // dest にファイルが現れている
        assert_eq!(read_to_string(&dest.join("hello.txt")), "hello fv");
        // src からは消えている
        assert!(!src.exists(), "src file should be gone after move");
    }
}
