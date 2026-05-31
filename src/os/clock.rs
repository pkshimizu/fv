//! ヘッダーに表示する現在の壁時計時刻（Clock）。
//! システム時計の読み取りという I/O と、表示用整形の純粋ロジックを分離し、後者をテスト可能にする。

use chrono::{DateTime, Datelike, Local, Timelike};

/// ローカル日時を `YYYY-MM-DD HH:MM:SS` 形式に整形する。
/// ファイル更新日時の表示（`VFileTime`）と同じ並びで、アプリ内の日時表記を統一する。
pub fn format_clock(dt: DateTime<Local>) -> String {
    format!(
        "{}-{:02}-{:02} {:02}:{:02}:{:02}",
        dt.year(),
        dt.month(),
        dt.day(),
        dt.hour(),
        dt.minute(),
        dt.second()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, TimeZone};

    #[test]
    fn format_clock_renders_date_and_time_zero_padded() {
        // 固定のローカル日時を組み立てれば、システムのタイムゾーンに依存せず決定論的に検証できる。
        let dt = Local.with_ymd_and_hms(2026, 6, 1, 9, 5, 3).unwrap();
        assert_eq!(format_clock(dt), "2026-06-01 09:05:03");
    }
}
