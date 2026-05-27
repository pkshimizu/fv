use crate::fs::VFile;
use crate::fs::file::{TOTAL_FILES_UNKNOWN, copy_files_with_progress, count_files};
use crate::state::ProgressMessage;
use std::any::Any;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;

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
        std::thread::spawn(move || match count_files(&files) {
            Ok(count) => {
                debug_assert_ne!(
                    count, TOTAL_FILES_UNKNOWN,
                    "count_files returned the sentinel value"
                );
                total_files.store(count, Ordering::Release);
            }
            Err(e) => {
                tracing::warn!("Failed to count files for copy progress: {e:#}");
            }
        });
    }

    let worker_cancel = Arc::clone(&cancel);
    let worker_total = Arc::clone(&total_files);
    std::thread::spawn(move || {
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            copy_files_with_progress(&files, &dest, &worker_cancel, &worker_total, |progress| {
                if tx.send(ProgressMessage::UpdateCopy(progress)).is_err() {
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
        // panic_any(value) で文字列以外のペイロードが投げられたケース。
        // TypeId の情報を含めることで開発者がパニック型を追跡できるようにする。
        format!(
            "<non-string panic payload: type_id={:?}>",
            payload.type_id()
        )
    }
}
