use crate::state::{Phase, ProgressMessage};
use anyhow::Result;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

/// Async Job 起動の戻り値。
/// Receiver で `ProgressMessage` を受信し、`cancel` を立てると worker は次の File-level Checkpoint で停止する。
pub struct AsyncJobHandle {
    pub rx: Receiver<ProgressMessage>,
    pub cancel: Arc<AtomicBool>,
}

/// Progress Update のスロットリング間隔。
/// zip エントリ数千件のような high-throughput な worker から大量の Update が
/// mpsc に滞留しないよう、最後の send からこの時間が経過するまでは送らない。
/// 初回 (processed == 0) と完了直前 (processed == total) は強制送出する。
const PROGRESS_TICK: Duration = Duration::from_millis(50);

/// Async Job を別スレッドで起動する。
/// クロージャは Cancel Token と進捗通知関数を受け取り、`anyhow::Result<()>` を返す。
/// Ok → `Complete`、Err → `Error(...)` を Receiver に流す。
///
/// 戻り値の `AsyncJobHandle` が drop され `Receiver` も drop されると、
/// 次の Update 送出時に自動的に Cancel Token が立ち worker は自然停止する。
/// この設計のため `JoinHandle` は意図的に保持せず detach する。
pub fn spawn_async_job<F>(f: F) -> AsyncJobHandle
where
    F: FnOnce(&AtomicBool, &mut dyn FnMut(Phase, usize, Option<usize>)) -> Result<()>
        + Send
        + 'static,
{
    let (tx, rx) = mpsc::channel::<ProgressMessage>();
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_cancel = cancel.clone();

    // Thread name を付けておくと panic 時にデバッガ / tracing でスレッドを識別しやすい。
    let spawn_result = std::thread::Builder::new()
        .name("fv-async-job".into())
        .spawn(move || {
            // on_progress クロージャを内側スコープに閉じ込めることで tx の借用を
            // catch_unwind 完了時に確実に解放し、その後 terminal メッセージ送出に使えるようにする。
            let outcome = {
                let progress_cancel = worker_cancel.clone();
                let mut last_send = Instant::now();
                let mut on_progress = |phase: Phase, processed: usize, total: Option<usize>| {
                    // 初回 / 完了直前 / PROGRESS_TICK 経過時のみ実送出する
                    let force = processed == 0
                        || total.is_some_and(|t| processed >= t)
                        || last_send.elapsed() >= PROGRESS_TICK;
                    if !force {
                        return;
                    }
                    let msg = ProgressMessage::Update {
                        phase,
                        processed,
                        total,
                    };
                    if tx.send(msg).is_err() {
                        // Receiver drop 検知。Cancel Token を立てて worker に終了を促す
                        progress_cancel.store(true, Ordering::Relaxed);
                        return;
                    }
                    last_send = Instant::now();
                };
                catch_unwind(AssertUnwindSafe(|| f(&worker_cancel, &mut on_progress)))
            };
            let terminal = match outcome {
                Ok(Ok(())) => ProgressMessage::Complete,
                // {e:#} で anyhow の context chain も含めて表示する
                Ok(Err(e)) => ProgressMessage::Error(format!("{e:#}")),
                Err(payload) => ProgressMessage::Error(panic_message(payload)),
            };
            // Receiver が既に drop されている場合は send が Err になるが、
            // その場合は誰も結果を待っていないので tracing に痕跡だけ残して捨てる
            if let Err(e) = tx.send(terminal) {
                tracing::warn!("async job terminal send failed: {e}");
            }
        });

    if let Err(e) = spawn_result {
        // OS スレッド生成失敗は異常状態。Receiver に Error を流して通常経路に合流させる
        let (err_tx, err_rx) = mpsc::channel::<ProgressMessage>();
        let _ = err_tx.send(ProgressMessage::Error(format!(
            "failed to spawn async job thread: {e}"
        )));
        return AsyncJobHandle { rx: err_rx, cancel };
    }

    AsyncJobHandle { rx, cancel }
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    let detail = payload
        .downcast_ref::<&'static str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str));
    match detail {
        Some(s) => format!("worker panic: {s}"),
        None => {
            tracing::warn!("worker panicked with non-string payload");
            "worker panic (non-string payload)".to_string()
        }
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
    fn spawn_async_job_forwards_forced_updates() {
        // 初回 (processed == 0) と完了直前 (processed == total) は強制送出される
        let handle = spawn_async_job(|_cancel, on_progress| {
            on_progress(Phase::Extracting, 0, Some(2));
            on_progress(Phase::Extracting, 2, Some(2));
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
                (Phase::Extracting, 0, Some(2)),
                (Phase::Extracting, 2, Some(2)),
            ]
        );
        assert!(matches!(msgs.last(), Some(ProgressMessage::Complete)));
    }

    #[test]
    fn spawn_async_job_coalesces_intermediate_updates_within_progress_tick() {
        // 同期的に連続発火した場合、PROGRESS_TICK 未満の中間値は捨てられる
        let handle = spawn_async_job(|_cancel, on_progress| {
            for i in 0..=100 {
                on_progress(Phase::Extracting, i, Some(100));
            }
            Ok(())
        });
        let msgs = recv_until_terminal(&handle.rx);
        let updates: Vec<_> = msgs
            .iter()
            .filter_map(|m| match m {
                ProgressMessage::Update { processed, .. } => Some(*processed),
                _ => None,
            })
            .collect();
        assert!(
            updates.contains(&0),
            "first Update (processed=0) must always be sent: {updates:?}"
        );
        assert!(
            updates.contains(&100),
            "terminal Update (processed=total) must always be sent: {updates:?}"
        );
        // 101 件すべては来ない (coalesce される)
        assert!(
            updates.len() < 101,
            "Updates must be coalesced; got {} of 101",
            updates.len()
        );
    }

    #[test]
    fn spawn_async_job_sends_complete_for_noop_worker() {
        let handle = spawn_async_job(|_cancel, _on_progress| Ok(()));
        let msgs = recv_until_terminal(&handle.rx);
        assert!(matches!(msgs.last(), Some(ProgressMessage::Complete)));
        assert_eq!(msgs.len(), 1);
    }

    #[test]
    fn spawn_async_job_auto_cancels_when_receiver_dropped() {
        use std::thread::sleep;

        let observed_cancel = Arc::new(AtomicBool::new(false));
        let observed_clone = observed_cancel.clone();
        let handle = spawn_async_job(move |cancel, on_progress| {
            for _ in 0..200 {
                if cancel.load(Ordering::Relaxed) {
                    observed_clone.store(true, Ordering::Relaxed);
                    return Ok(());
                }
                // PROGRESS_TICK を確実に超える間隔で送出してテストを安定化
                on_progress(Phase::Extracting, 0, Some(1));
                sleep(Duration::from_millis(60));
            }
            Ok(())
        });

        drop(handle.rx);

        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while !observed_cancel.load(Ordering::Relaxed) {
            if std::time::Instant::now() > deadline {
                panic!("worker never observed cancel after Receiver drop");
            }
            sleep(Duration::from_millis(20));
        }
        assert!(handle.cancel.load(Ordering::Relaxed));
    }

    #[test]
    fn spawn_async_job_sends_error_when_worker_panics() {
        let handle = spawn_async_job(|_cancel, _on_progress| {
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
