//! Phase ループの低層 primitive 群。Operation Phase のキャンセル可能ループ
//! （`for_each_until_cancelled` / `process_items`）と Scan Phase の進捗バッチ通知
//! （`notify_scan_progress`）、および両 Phase 共通の終了理由 `CollectStatus` を提供する。

use crate::state::Phase;
use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};

/// Scan Phase 中、ファイル発見ごとに `on_progress` を呼ばずに、この件数ごとにバッチで通知する。
/// `&mut dyn FnMut` の vtable hop と `spawn_async_job` 側の `Instant::now()` 呼出を削減する目的。
pub(super) const SCAN_NOTIFY_BATCH: usize = 256;

/// cancel 可能なループ（Scan Phase の走査・Operation Phase の処理いずれも）の終了理由。
/// `Completed` = 全件やり切った / `Cancelled` = Cancel Token により途中で打ち切った。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CollectStatus {
    Completed,
    Cancelled,
}

impl CollectStatus {
    pub(super) fn is_cancelled(self) -> bool {
        matches!(self, CollectStatus::Cancelled)
    }
}

/// Operation Phase の「File-level Checkpoint」を単一化した低層プリミティブ。
/// 各要素の処理前に `cancel` を見て、立っていれば `Cancelled` で早期中断する。
/// `op` が `Err` を返しても `cancel` が立っていれば「キャンセル優先」として `Cancelled` に畳む
/// （ユーザが停止を要求済みなら、停止直前のレース起因エラーは表に出さず Partial Result を残す）。
/// 全件完走で `Completed`。
///
/// 同一FS Move（`run_move` の rename 高速パス）と Zip 解凍（`run_zip_extract`）は、
/// 前者が probe 済み先頭要素を `skip(1)` する不規則な件数進行、後者が `&[T]` でなく
/// `archive` を index 走査する構造のため、この標準形には寄せず bespoke のまま残している。
pub(super) fn for_each_until_cancelled<T>(
    items: &[T],
    cancel: &AtomicBool,
    mut op: impl FnMut(&T) -> Result<()>,
) -> Result<CollectStatus> {
    for item in items {
        if cancel.load(Ordering::Relaxed) {
            return Ok(CollectStatus::Cancelled);
        }
        if let Err(e) = op(item) {
            if cancel.load(Ordering::Relaxed) {
                return Ok(CollectStatus::Cancelled);
            }
            return Err(e);
        }
    }
    Ok(CollectStatus::Completed)
}

/// `for_each_until_cancelled` に進捗送出を足した Operation Phase の標準ループ。
/// 各要素の処理成功ごとに `(phase, 処理済み件数, Some(items.len()))` を emit する。
/// 開始時の `(phase, 0, total)` 通知は呼び出し側が担う（mkdir 等を挟む前に phase を
/// 切り替えるため）。早期中断で `Cancelled`、全件完走で `Completed`。
pub(super) fn process_items<T>(
    items: &[T],
    phase: Phase,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
    mut op: impl FnMut(&T) -> Result<()>,
) -> Result<CollectStatus> {
    let total = items.len();
    let mut done = 0;
    for_each_until_cancelled(items, cancel, |item| {
        op(item)?;
        done += 1;
        on_progress(phase, done, Some(total));
        Ok(())
    })
}

/// Scan Phase の batch 通知ヘルパ。
/// `count == 0` での発火 (空入力でカウント前の状態) を抑止しつつ、
/// `SCAN_NOTIFY_BATCH` の倍数到達時に `(Scanning, count, None)` を発火する。
pub(super) fn notify_scan_progress(
    count: usize,
    on_progress: &mut dyn FnMut(Phase, usize, Option<usize>),
) {
    if count > 0 && count.is_multiple_of(SCAN_NOTIFY_BATCH) {
        on_progress(Phase::Scanning, count, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn for_each_until_cancelled_runs_op_for_every_item_and_returns_completed() {
        let items = [1, 2, 3];
        let cancel = AtomicBool::new(false);
        let mut seen = Vec::new();

        let outcome = for_each_until_cancelled(&items, &cancel, |&n| {
            seen.push(n);
            Ok(())
        })
        .expect("no error");

        assert_eq!(outcome, CollectStatus::Completed, "all items processed");
        assert_eq!(seen, vec![1, 2, 3]);
    }

    #[test]
    fn for_each_until_cancelled_stops_before_op_when_cancel_is_preset() {
        let items = [1, 2, 3];
        let cancel = AtomicBool::new(true);
        let mut called = false;

        let outcome = for_each_until_cancelled(&items, &cancel, |_| {
            called = true;
            Ok(())
        })
        .expect("preset cancel is not an error");

        assert_eq!(
            outcome,
            CollectStatus::Cancelled,
            "preset cancel → Cancelled"
        );
        assert!(!called, "op must not run when cancel is preset");
    }

    #[test]
    fn for_each_until_cancelled_propagates_op_error_when_not_cancelled() {
        let items = [1, 2, 3];
        let cancel = AtomicBool::new(false);

        let result = for_each_until_cancelled(&items, &cancel, |&n| {
            if n == 2 {
                anyhow::bail!("boom at {n}");
            }
            Ok(())
        });

        let err = result.expect_err("op error propagates when not cancelled");
        assert!(format!("{err:#}").contains("boom at 2"));
    }

    #[test]
    fn for_each_until_cancelled_folds_op_error_into_cancel_when_cancel_is_set() {
        let items = [1, 2, 3];
        let cancel = AtomicBool::new(false);

        // op が「cancel を立てつつ Err を返す」レースを再現する。
        let outcome = for_each_until_cancelled(&items, &cancel, |&n| {
            if n == 2 {
                cancel.store(true, Ordering::Relaxed);
                anyhow::bail!("error coincides with cancel");
            }
            Ok(())
        })
        .expect("error coinciding with cancel is folded into cancel, not surfaced");

        assert_eq!(
            outcome,
            CollectStatus::Cancelled,
            "folded cancel → Cancelled"
        );
    }

    #[test]
    fn process_items_emits_progress_per_success_with_len_as_total() {
        let items = ["a", "b", "c"];
        let cancel = AtomicBool::new(false);
        let mut events = Vec::new();

        let outcome = process_items(
            &items,
            Phase::Copying,
            &cancel,
            &mut |phase, done, total| events.push((phase, done, total)),
            |_| Ok(()),
        )
        .expect("no error");

        assert_eq!(outcome, CollectStatus::Completed, "all items processed");
        assert_eq!(
            events,
            vec![
                (Phase::Copying, 1, Some(3)),
                (Phase::Copying, 2, Some(3)),
                (Phase::Copying, 3, Some(3)),
            ]
        );
    }
}
