use crate::fs::VFile;
use crate::fs::file::{CopyProgress, copy_files_with_progress};
use crate::fs::format::format_bytes;
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
                let _ = tx.send(ProgressMessage::Error(e));
            }
            Err(payload) => {
                let _ = tx.send(ProgressMessage::Error(anyhow::anyhow!(
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
