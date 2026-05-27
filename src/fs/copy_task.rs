use crate::fs::VFile;
use crate::fs::file::{CopyProgress, copy_files_with_progress, count_files};
use crate::fs::format::format_bytes;
use crate::state::{ProgressFormatter, ProgressMessage};
use std::any::Any;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;

/// 総ファイル数がまだ確定していないことを表すセンチネル値。
/// `copy_files_with_progress` 側の定数と同じ意味で重複定義しているが、
/// 公開する atomic の生成タイミングをここで一元管理する目的で再宣言している。
const TOTAL_FILES_UNKNOWN: usize = usize::MAX;

/// 非同期コピー処理のハンドル。
/// `rx` で進捗・完了・エラーを受信し、`cancel` を `true` に設定すれば外部からコピーを中断できる。
pub struct CopyHandle {
    pub rx: mpsc::Receiver<ProgressMessage>,
    pub cancel: Arc<AtomicBool>,
}

/// バックグラウンドスレッドで `files` を `dest` へ非同期コピーし、
/// 進捗・完了・エラーを `ProgressMessage` として返却 receiver へ流す。
/// 受信側が drop されるか `CopyHandle::cancel` が `true` になると次のチャンク読み込み時にコピーが中断される。
/// 総ファイル数の集計は別スレッド（detach）で実行し、コピー本体の完了を引き伸ばさない。
/// スレッド内で panic が発生しても `catch_unwind` で捕捉し、エラーとして UI に通知する。
pub fn spawn_copy_files(files: Vec<VFile>, dest: String) -> CopyHandle {
    let (tx, rx) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let total_files = Arc::new(AtomicUsize::new(TOTAL_FILES_UNKNOWN));
    let files = Arc::new(files);

    // バックグラウンドの detached スレッドで総ファイル数を集計する。
    // ここでの失敗は UI 上「分母が ? のまま」になるだけで、コピー本体には影響しない。
    {
        let files = Arc::clone(&files);
        let total_files = Arc::clone(&total_files);
        std::thread::spawn(move || {
            if let Ok(count) = count_files(&files) {
                total_files.store(count, Ordering::Release);
            }
        });
    }

    let worker_cancel = Arc::clone(&cancel);
    let worker_total = Arc::clone(&total_files);
    std::thread::spawn(move || {
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            copy_files_with_progress(&files, &dest, &worker_cancel, &worker_total, |progress| {
                if tx
                    .send(ProgressMessage::Update(Box::new(progress)))
                    .is_err()
                {
                    // 受信側が消えた。コピーを早期中断する。
                    worker_cancel.store(true, Ordering::Relaxed);
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

    CopyHandle { rx, cancel }
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
