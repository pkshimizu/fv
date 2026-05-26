use crate::fs::VFile;
use crate::fs::file::{CopyProgress, copy_files_with_progress};
use crate::state::{ProgressFormatter, ProgressMessage};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

/// バックグラウンドスレッドで `files` を `dest` へ非同期コピーし、
/// 進捗・完了・エラーを `ProgressMessage` として返却 receiver へ流す。
/// 受信側が drop されると次のチャンク読み込み時にコピーが中断される。
pub fn spawn_copy_files(files: Vec<VFile>, dest: String) -> mpsc::Receiver<ProgressMessage> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let cancel = AtomicBool::new(false);
        let result = copy_files_with_progress(&files, &dest, &cancel, |progress| {
            if tx
                .send(ProgressMessage::Update(Box::new(progress)))
                .is_err()
            {
                // 受信側が消えた。コピーを早期中断する。
                cancel.store(true, Ordering::Relaxed);
            }
        });
        match result {
            Ok(()) => {
                let _ = tx.send(ProgressMessage::Complete);
            }
            Err(e) => {
                let _ = tx.send(ProgressMessage::Error(format!("{e:#}")));
            }
        }
    });

    rx
}

impl ProgressFormatter for CopyProgress {
    fn format(&self) -> String {
        let total = match self.total_files {
            Some(n) => n.to_string(),
            None => "?".to_string(),
        };
        format!(
            "Copying {}/{} files  {} / {}",
            self.copied_files,
            total,
            format_bytes(self.current_bytes),
            format_bytes(self.current_total_bytes),
        )
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}
