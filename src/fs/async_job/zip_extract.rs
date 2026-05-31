//! Zip 解凍 Job。archive を index 走査し、各エントリを dest に展開する（Scan Phase なし）。

use crate::fs::VFile;
use crate::state::Phase;
use anyhow::{Context, Result};
use std::io::BufWriter;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

pub(super) fn run_zip_extract(
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
    use crate::fs::async_job::FileJob;
    use crate::fs::async_job::test_support::{build_sample_zip, read_to_string, vfile};
    use tempfile::TempDir;

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
}
