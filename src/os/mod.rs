//! ホスト OS／環境との対話（ファイルシステムを超えた領域）。
//! クリップボード書き込みやシステム情報取得など、`fs`（ファイル操作）に属さない
//! ホスト環境とのやり取りをここに集約する。

use std::time::{Duration, Instant};

pub mod clipboard;
pub mod clock;
pub mod disk_usage;
pub mod system_info;

/// 動的情報を再取得する壁時計間隔。System Info / Disk Usage が共有する。
pub(crate) const REFRESH_INTERVAL: Duration = Duration::from_secs(5);

/// 前回リフレッシュからの経過時間で再取得可否を返す壁時計ベースのスロットル。
/// tick の呼び出し回数（=メインループの反復回数や入力量）に依存せず、真に `interval`
/// 間隔で `true` を返す。System Info / Disk Usage が同じ間隔で再取得するために共有する
/// （どちらにも属さない中立な型）。
pub(crate) struct RefreshThrottle {
    last: Instant,
    interval: Duration,
}

impl RefreshThrottle {
    pub(crate) fn new(interval: Duration) -> Self {
        Self {
            last: Instant::now(),
            interval,
        }
    }

    /// 現在時刻で判定する。前回リフレッシュから `interval` 以上経過していれば `true` を返し、
    /// 基準時刻をリセットする。
    pub(crate) fn tick(&mut self) -> bool {
        self.tick_at(Instant::now())
    }

    /// `tick` の本体。現在時刻を引数で受け取り、時間依存ロジックを決定的にテスト可能にする。
    /// `saturating_duration_since` を使い、万一 `now < last`（時刻巻き戻り等）でも panic せず
    /// `Duration::ZERO` 扱いとする。
    fn tick_at(&mut self, now: Instant) -> bool {
        if now.saturating_duration_since(self.last) >= self.interval {
            self.last = now;
            true
        } else {
            false
        }
    }
}

/// 使用量/総容量を `used/totalUNIT`（小数1桁）に整形する。Mem・Disk 共通。
/// 単位は総容量に応じて選ぶ（1TiB 以上なら T、それ未満は G）。used も同じ単位で表す。
pub(crate) fn format_used_total(used: u64, total: u64) -> String {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    const TIB: f64 = 1024.0 * GIB;
    let (divisor, unit) = if total as f64 >= TIB {
        (TIB, "T")
    } else {
        (GIB, "G")
    };
    format!(
        "{:.1}/{:.1}{unit}",
        used as f64 / divisor,
        total as f64 / divisor
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_throttle_fires_once_interval_elapses_and_resets() {
        let interval = Duration::from_secs(5);
        let mut throttle = RefreshThrottle::new(interval);
        let base = throttle.last;

        // interval 未満は false。
        assert!(!throttle.tick_at(base + Duration::from_secs(4)));
        // interval ちょうどで true（基準時刻をリセット）。
        assert!(throttle.tick_at(base + interval));
        // リセット直後は再び interval 未満なので false。
        assert!(!throttle.tick_at(base + interval + Duration::from_secs(4)));
        // 前回 true の時刻から interval 経過で再び true。
        assert!(throttle.tick_at(base + interval + interval));
    }

    #[test]
    fn format_used_total_renders_gib_with_one_decimal() {
        assert_eq!(
            format_used_total(8_804_682_956, 17_179_869_184),
            "8.2/16.0G"
        );
    }

    #[test]
    fn format_used_total_scales_to_terabytes_by_total() {
        const GIB: u64 = 1024 * 1024 * 1024;
        assert_eq!(format_used_total(1024 * GIB, 2 * 1024 * GIB), "1.0/2.0T");
    }
}
