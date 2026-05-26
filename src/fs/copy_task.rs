use crate::fs::VFile;
use crate::fs::file::{CopyProgress, copy_files_with_progress};
use crate::state::{ProgressFormatter, ProgressMessage};
use std::any::Any;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

/// バックグラウンドスレッドで `files` を `dest` へ非同期コピーし、
/// 進捗・完了・エラーを `ProgressMessage` として返却 receiver へ流す。
/// 受信側が drop されると次のチャンク読み込み時にコピーが中断される。
/// スレッド内で panic が発生しても `catch_unwind` で捕捉し、エラーとして UI に通知する。
pub fn spawn_copy_files(files: Vec<VFile>, dest: String) -> mpsc::Receiver<ProgressMessage> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            let cancel = AtomicBool::new(false);
            copy_files_with_progress(&files, &dest, &cancel, |progress| {
                if tx
                    .send(ProgressMessage::Update(Box::new(progress)))
                    .is_err()
                {
                    // 受信側が消えた。コピーを早期中断する。
                    cancel.store(true, Ordering::Relaxed);
                }
            })
        }));
        match result {
            Ok(Ok(())) => {
                let _ = tx.send(ProgressMessage::Complete);
            }
            Ok(Err(e)) => {
                let _ = tx.send(ProgressMessage::Error(format!("{e:#}")));
            }
            Err(payload) => {
                let _ = tx.send(ProgressMessage::Error(format!(
                    "Copy thread panicked: {}",
                    panic_message(&*payload)
                )));
            }
        }
    });

    rx
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
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
