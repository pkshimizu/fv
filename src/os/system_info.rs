//! ホスト環境のシステム情報（OS・カーネル・ホスト名・CPU・メモリ・アップタイム）。
//! `sysinfo` クレートをこのモジュールに閉じ込め、表示用の `SystemInfo` 値型として公開する。

use sysinfo::System;

/// `sysinfo::System` を保持し、静的情報を 1 回・動的情報を約1秒ごとに取得する。
/// 呼び出し側（AppContext / ヘッダー）は sysinfo を直接触らず、本 reader 越しに `SystemInfo` を読む。
pub(crate) struct SystemInfoReader {
    system: System,
    throttle: RefreshThrottle,
    current: SystemInfo,
}

impl SystemInfoReader {
    pub(crate) fn new() -> Self {
        let mut reader = Self {
            system: System::new(),
            throttle: RefreshThrottle::new(),
            current: Self::gather_static(),
        };
        // 初回の動的情報を埋める。CPU 使用率は前回サンプルとの差分で算出されるため、
        // この初回 refresh 直後の値は不正確（0% 付近）で、最初の tick による 2 回目の
        // refresh（約1秒後）で正常な値に落ち着く。起動直後の一瞬だけ低めに出る点は許容する。
        reader.refresh_dynamic();
        reader
    }

    /// 1 tick 進める。スロットルが許せば動的情報（CPU・メモリ・アップタイム）を再取得する。
    /// 静的情報（OS/カーネル/ホスト名）は変化しないので再取得しない。
    pub(crate) fn tick(&mut self) {
        if self.throttle.tick() {
            self.refresh_dynamic();
        }
    }

    /// 動的情報（CPU・メモリ・アップタイム）のみ再取得する。静的情報は触らない。
    fn refresh_dynamic(&mut self) {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.current.cpu_percent = self.system.global_cpu_usage();
        self.current.mem_used = self.system.used_memory();
        self.current.mem_total = self.system.total_memory();
        self.current.uptime_secs = System::uptime();
    }

    pub(crate) fn current(&self) -> &SystemInfo {
        &self.current
    }

    /// 静的情報（OS/カーネル/ホスト名）を取得する。動的フィールドは 0 で初期化し、
    /// 直後の `refresh_dynamic` で埋める。
    fn gather_static() -> SystemInfo {
        let os_version = System::os_version().unwrap_or_default();
        // long_os_version 例: "macOS 26.5"。末尾の os_version を除いて製品名 "macOS" を取り出す。
        // os_version が空のときは strip_suffix("") が全体一致してしまうため、その場合は long をそのまま使う。
        let long = System::long_os_version().unwrap_or_default();
        let os_name = if os_version.is_empty() {
            long.clone()
        } else {
            long.strip_suffix(&os_version)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or(long.as_str())
                .to_string()
        };
        SystemInfo {
            os_name,
            os_version,
            // sysinfo の name() は macOS で "Darwin" 等のカーネル名を返す。
            kernel_name: System::name().unwrap_or_default(),
            kernel_version: System::kernel_version().unwrap_or_default(),
            hostname: System::host_name().unwrap_or_default(),
            cpu_percent: 0.0,
            mem_used: 0,
            mem_total: 0,
            uptime_secs: 0,
        }
    }
}

/// ヘッダーに表示するホスト環境のスナップショット。
/// 静的フィールド（OS/カーネル/ホスト名）はタイトルへ、動的フィールド（CPU/メモリ/アップタイム）は
/// 内容行へ整形する。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SystemInfo {
    pub(crate) os_name: String,
    pub(crate) os_version: String,
    pub(crate) kernel_name: String,
    pub(crate) kernel_version: String,
    pub(crate) hostname: String,
    pub(crate) cpu_percent: f32,
    pub(crate) mem_used: u64,
    pub(crate) mem_total: u64,
    pub(crate) uptime_secs: u64,
}

impl SystemInfo {
    /// ヘッダータイトルに載せる静的情報を ` | ` 区切りで整形する。
    /// `name<version>` 形式（OS・カーネル）＋末尾にホスト名（ブラケットなし）。
    /// 例: `macOS<15.5> | Darwin<24.5.0> | kenji-mac`
    pub(crate) fn title_fields(&self) -> String {
        format!(
            "{}<{}> | {}<{}> | {}",
            self.os_name, self.os_version, self.kernel_name, self.kernel_version, self.hostname
        )
    }

    /// ヘッダー内容行に載せる動的情報を整形する。フィールド間はダブルスペース区切り。
    /// 例: `CPU 12%  Mem 8.2/16.0G  up 3h21m`
    pub(crate) fn status_line(&self) -> String {
        format!(
            "CPU {:.0}%  Mem {}  up {}",
            self.cpu_percent,
            format_mem(self.mem_used, self.mem_total),
            format_uptime(self.uptime_secs),
        )
    }
}

/// 動的情報を再取得する間隔（tick 数）。tick は約250ms なので約1秒ごと。
const REFRESH_EVERY_N_TICKS: u32 = 4;

/// tick を数えて `REFRESH_EVERY_N_TICKS` ごとに 1 回だけリフレッシュ可否を返す簡易スロットル。
struct RefreshThrottle {
    ticks: u32,
}

impl RefreshThrottle {
    fn new() -> Self {
        Self { ticks: 0 }
    }

    /// 1 tick 進める。リフレッシュすべきタイミングなら `true` を返してカウンタをリセットする。
    fn tick(&mut self) -> bool {
        self.ticks += 1;
        if self.ticks >= REFRESH_EVERY_N_TICKS {
            self.ticks = 0;
            true
        } else {
            false
        }
    }
}

/// 使用量/総容量を GiB 単位・小数1桁で `used/totalG` に整形する。
fn format_mem(used: u64, total: u64) -> String {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    format!("{:.1}/{:.1}G", used as f64 / GIB, total as f64 / GIB)
}

/// 秒数を `{h}h{m}m`（1時間未満は `{m}m`、1日以上は `{d}d{h}h`）に整形する。
fn format_uptime(secs: u64) -> String {
    let days = secs / 86_400;
    let hours = (secs % 86_400) / 3_600;
    let minutes = (secs % 3_600) / 60;
    if days > 0 {
        format!("{days}d{hours}h")
    } else if hours > 0 {
        format!("{hours}h{minutes}m")
    } else {
        format!("{minutes}m")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> SystemInfo {
        SystemInfo {
            os_name: "macOS".to_string(),
            os_version: "15.5".to_string(),
            kernel_name: "Darwin".to_string(),
            kernel_version: "24.5.0".to_string(),
            hostname: "kenji-mac".to_string(),
            cpu_percent: 12.0,
            mem_used: 8_804_682_956,
            mem_total: 17_179_869_184,
            uptime_secs: 12_081,
        }
    }

    #[test]
    fn title_fields_joins_static_info_with_pipe_and_name_version_form() {
        assert_eq!(
            sample().title_fields(),
            "macOS<15.5> | Darwin<24.5.0> | kenji-mac"
        );
    }

    #[test]
    fn status_line_joins_dynamic_info_with_spaces() {
        assert_eq!(sample().status_line(), "CPU 12%  Mem 8.2/16.0G  up 3h21m");
    }

    #[test]
    fn format_uptime_under_one_hour_shows_minutes_only() {
        assert_eq!(format_uptime(21 * 60 + 30), "21m");
    }

    #[test]
    fn format_uptime_one_day_or_more_shows_days_and_hours() {
        assert_eq!(format_uptime(86_400 + 3 * 3_600 + 21 * 60), "1d3h");
    }

    #[test]
    fn format_mem_renders_gib_with_one_decimal() {
        assert_eq!(format_mem(8_804_682_956, 17_179_869_184), "8.2/16.0G");
    }

    #[test]
    fn refresh_throttle_fires_every_fourth_tick_and_resets() {
        let mut throttle = RefreshThrottle::new();
        // tick は約250ms。1秒（4tick）ごとに 1 回だけ true。
        assert_eq!(
            (0..8).map(|_| throttle.tick()).collect::<Vec<_>>(),
            vec![false, false, false, true, false, false, false, true]
        );
    }
}
