use crate::fs::VFile;
use crate::state::Phase;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

mod checkpoint;
mod copy;
mod delete;
mod destination;
mod move_op;
mod plan;
#[cfg(test)]
mod test_support;
mod zip_create;
mod zip_extract;

use copy::run_copy;
use delete::run_delete;
use move_op::run_move;
use zip_create::run_zip_create;
use zip_extract::run_zip_extract;

/// Async Job として実行される重いファイル操作。
/// UI とは結合せず、進捗はクロージャ経由で通知する。
/// `Phase::Cancelling` は worker からは emit せず、UI 側で Esc 受信時に上書きされる。
#[derive(Debug)]
pub enum FileJob {
    ZipExtract {
        file: VFile,
        dest: PathBuf,
    },
    Copy {
        files: Vec<VFile>,
        dest: PathBuf,
    },
    Move {
        files: Vec<VFile>,
        dest: PathBuf,
    },
    ZipCreate {
        dir: VFile,
        name: String,
        files: Vec<VFile>,
    },
    Delete {
        files: Vec<VFile>,
    },
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
            FileJob::ZipExtract { file, dest } => {
                run_zip_extract(&file, &dest, cancel, on_progress)
            }
            FileJob::Copy { files, dest } => run_copy(&files, &dest, cancel, on_progress),
            FileJob::Move { files, dest } => run_move(&files, &dest, cancel, on_progress),
            FileJob::ZipCreate { dir, name, files } => {
                run_zip_create(&dir, &name, &files, cancel, on_progress)
            }
            FileJob::Delete { files } => run_delete(&files, cancel, on_progress),
        }
    }
}
