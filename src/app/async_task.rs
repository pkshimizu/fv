use crate::state::{Phase, ProgressMessage};
use anyhow::Result;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};

/// Async Job 起動の戻り値。
/// Receiver で `ProgressMessage` を受信し、`cancel` を立てると worker は次の File-level Checkpoint で停止する。
pub struct AsyncTaskHandle {
    pub rx: Receiver<ProgressMessage>,
    pub cancel: Arc<AtomicBool>,
}

/// Async Job を別スレッドで起動する。
/// クロージャは `cancel` と進捗通知関数を受け取り、`Result<()>` を返す。
/// Ok → `Complete`、Err → `Error(...)` を Receiver に流す。
pub fn spawn_async_task<F>(f: F) -> AsyncTaskHandle
where
    F: FnOnce(&AtomicBool, &mut dyn FnMut(Phase, usize, Option<usize>)) -> Result<()>
        + Send
        + 'static,
{
    let (tx, rx) = mpsc::channel::<ProgressMessage>();
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_cancel = cancel.clone();
    let worker_tx = tx.clone();

    let progress_cancel = cancel.clone();
    std::thread::spawn(move || {
        let mut on_progress = |phase: Phase, processed: usize, total: Option<usize>| {
            let send_result = worker_tx.send(ProgressMessage::Update {
                phase,
                processed,
                total,
            });
            // Receiver が drop されていた場合は cancel を立てて worker に終了を促す
            if send_result.is_err() {
                progress_cancel.store(true, Ordering::Release);
            }
        };
        let outcome = catch_unwind(AssertUnwindSafe(|| f(&worker_cancel, &mut on_progress)));
        let terminal = match outcome {
            Ok(Ok(())) => ProgressMessage::Complete,
            Ok(Err(e)) => ProgressMessage::Error(format!("{e}")),
            Err(payload) => ProgressMessage::Error(panic_message(payload)),
        };
        let _ = worker_tx.send(terminal);
    });

    AsyncTaskHandle { rx, cancel }
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        format!("worker panic: {s}")
    } else if let Some(s) = payload.downcast_ref::<String>() {
        format!("worker panic: {s}")
    } else {
        "worker panic (non-string payload)".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::RecvTimeoutError;
    use std::time::Duration;

    fn recv_until_terminal(rx: &Receiver<ProgressMessage>) -> Vec<ProgressMessage> {
        let mut out = Vec::new();
        loop {
            match rx.recv_timeout(Duration::from_secs(2)) {
                Ok(msg @ ProgressMessage::Complete) | Ok(msg @ ProgressMessage::Error(_)) => {
                    out.push(msg);
                    return out;
                }
                Ok(msg) => out.push(msg),
                Err(RecvTimeoutError::Timeout) => panic!("worker did not finish within timeout"),
                Err(RecvTimeoutError::Disconnected) => return out,
            }
        }
    }

    #[test]
    fn spawn_async_task_forwards_on_progress_calls_as_update_messages() {
        let handle = spawn_async_task(|_cancel, on_progress| {
            on_progress(Phase::Extracting, 7, Some(50));
            on_progress(Phase::Extracting, 8, Some(50));
            Ok(())
        });
        let msgs = recv_until_terminal(&handle.rx);
        let updates: Vec<_> = msgs
            .iter()
            .filter_map(|m| match m {
                ProgressMessage::Update {
                    phase,
                    processed,
                    total,
                } => Some((*phase, *processed, *total)),
                _ => None,
            })
            .collect();
        assert_eq!(
            updates,
            vec![
                (Phase::Extracting, 7, Some(50)),
                (Phase::Extracting, 8, Some(50)),
            ]
        );
        assert!(matches!(msgs.last(), Some(ProgressMessage::Complete)));
    }

    #[test]
    fn spawn_async_task_sends_complete_for_noop_worker() {
        let handle = spawn_async_task(|_cancel, _on_progress| Ok(()));
        let msgs = recv_until_terminal(&handle.rx);
        assert!(matches!(msgs.last(), Some(ProgressMessage::Complete)));
        assert_eq!(msgs.len(), 1);
    }

    #[test]
    fn spawn_async_task_auto_cancels_when_receiver_dropped() {
        use std::sync::atomic::Ordering;
        use std::thread::sleep;

        let observed_cancel = Arc::new(AtomicBool::new(false));
        let observed_clone = observed_cancel.clone();
        let handle = spawn_async_task(move |cancel, on_progress| {
            for _ in 0..200 {
                if cancel.load(Ordering::Acquire) {
                    observed_clone.store(true, Ordering::Release);
                    return Ok(());
                }
                on_progress(Phase::Extracting, 0, Some(1));
                sleep(Duration::from_millis(20));
            }
            Ok(())
        });

        // Receiver を drop → tx.send() が以後失敗 → spawn_async_task 内部で cancel が立つ
        drop(handle.rx);

        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while !observed_cancel.load(Ordering::Acquire) {
            if std::time::Instant::now() > deadline {
                panic!("worker never observed cancel after Receiver drop");
            }
            sleep(Duration::from_millis(20));
        }
        // 外部に公開されている cancel ハンドルも true になっている
        assert!(handle.cancel.load(Ordering::Acquire));
    }

    #[test]
    fn spawn_async_task_sends_error_when_worker_panics() {
        let handle = spawn_async_task(|_cancel, _on_progress| {
            panic!("boom from worker");
        });
        let msgs = recv_until_terminal(&handle.rx);
        let last = msgs.last().expect("must terminate");
        match last {
            ProgressMessage::Error(text) => {
                assert!(
                    text.contains("boom from worker") || text.contains("panic"),
                    "Error payload should mention the panic: {text}"
                );
            }
            other => panic!("expected Error(_), got {other:?}"),
        }
    }
}
