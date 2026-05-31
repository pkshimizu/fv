//! ホスト OS／環境との対話（ファイルシステムを超えた領域）。
//! クリップボード書き込みやシステム情報取得など、`fs`（ファイル操作）に属さない
//! ホスト環境とのやり取りをここに集約する。

pub mod clipboard;
pub mod clock;
pub mod disk_usage;
pub mod system_info;

/// 動的情報を再取得する間隔（tick 数）。tick は約250ms なので約1秒ごと。
const REFRESH_EVERY_N_TICKS: u32 = 4;

/// tick を数えて `REFRESH_EVERY_N_TICKS` ごとに 1 回だけリフレッシュ可否を返す簡易スロットル。
/// System Info / Disk Usage が同じ約1秒間隔で再取得するために共有する（どちらにも属さない中立な型）。
pub(crate) struct RefreshThrottle {
    ticks: u32,
}

impl RefreshThrottle {
    pub(crate) fn new() -> Self {
        Self { ticks: 0 }
    }

    /// 1 tick 進める。リフレッシュすべきタイミングなら `true` を返してカウンタをリセットする。
    pub(crate) fn tick(&mut self) -> bool {
        self.ticks += 1;
        if self.ticks >= REFRESH_EVERY_N_TICKS {
            self.ticks = 0;
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
    fn refresh_throttle_fires_every_fourth_tick_and_resets() {
        let mut throttle = RefreshThrottle::new();
        // tick は約250ms。1秒（4tick）ごとに 1 回だけ true。
        assert_eq!(
            (0..8).map(|_| throttle.tick()).collect::<Vec<_>>(),
            vec![false, false, false, true, false, false, false, true]
        );
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
