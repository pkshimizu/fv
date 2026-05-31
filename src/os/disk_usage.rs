//! カレントディレクトリが属するストレージボリュームの使用状況（Disk Usage）。
//! sysinfo 依存の I/O（`DiskUsageReader`）と、ディレクトリ→ボリューム照合・整形の
//! 純粋ロジックを分離し、後者をユニットテスト可能にする。

use crate::os::system_info::RefreshThrottle;
use std::path::Path;
use std::path::PathBuf;
use sysinfo::Disks;

/// マウント済みボリュームの容量情報（sysinfo の 1 ディスク分に相当）。
pub struct Volume {
    pub mount_point: PathBuf,
    pub total: u64,
    pub available: u64,
}

/// あるディレクトリが属するボリュームの使用量と総容量。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiskUsage {
    pub used: u64,
    pub total: u64,
}

impl DiskUsage {
    /// 使用率（%）を四捨五入で返す。total が 0 のときは 0。
    pub fn percent(&self) -> u64 {
        if self.total == 0 {
            return 0;
        }
        (self.used as f64 / self.total as f64 * 100.0).round() as u64
    }
}

/// `dir` を含むボリュームをマウントポイントの最長プレフィックス一致で解決する。
/// `used = total - available`。一致するボリュームが無ければ `None`。
pub fn resolve(dir: &Path, volumes: &[Volume]) -> Option<DiskUsage> {
    volumes
        .iter()
        .filter(|v| dir.starts_with(&v.mount_point))
        .max_by_key(|v| v.mount_point.as_os_str().len())
        .map(|v| DiskUsage {
            used: v.total.saturating_sub(v.available),
            total: v.total,
        })
}

/// sysinfo の `Disks` を保持し、約1秒ごとに容量を再取得する。
/// 呼び出し側（ヘッダー）は sysinfo を直接触らず、本 reader 越しにカレントディレクトリの
/// `DiskUsage` を読む。System Info reader と同型。
pub(crate) struct DiskUsageReader {
    disks: Disks,
    throttle: RefreshThrottle,
}

impl DiskUsageReader {
    pub(crate) fn new() -> Self {
        Self {
            disks: Disks::new_with_refreshed_list(),
            throttle: RefreshThrottle::new(),
        }
    }

    /// 1 tick 進める。スロットルが許せばボリューム一覧の容量を再取得する。
    pub(crate) fn tick(&mut self) {
        if self.throttle.tick() {
            self.disks.refresh(true);
        }
    }

    /// `dir` が属するボリュームの使用状況を返す。特定できなければ `None`。
    pub(crate) fn usage_for(&self, dir: &Path) -> Option<DiskUsage> {
        let volumes: Vec<Volume> = self
            .disks
            .list()
            .iter()
            .map(|d| Volume {
                mount_point: d.mount_point().to_path_buf(),
                total: d.total_space(),
                available: d.available_space(),
            })
            .collect();
        resolve(dir, &volumes)
    }
}

/// ヘッダーに載せる Disk フィールド文字列を整形する。
/// 使用量/総容量(使用率) 形式。例: `Disk 120.0/500.0G (24%)`。
/// ボリューム未特定（`None`）のときは `Disk n/a`。
pub fn format_disk_field(usage: Option<DiskUsage>) -> String {
    match usage {
        Some(u) => format!(
            "Disk {} ({}%)",
            format_disk_sizes(u.used, u.total),
            u.percent()
        ),
        None => "Disk n/a".to_string(),
    }
}

/// 使用量/総容量を `used/totalUNIT`（小数1桁）に整形する。
/// 単位は総容量に応じて選ぶ（1TiB 以上なら T、それ未満は G）。used も同じ単位で表す。
fn format_disk_sizes(used: u64, total: u64) -> String {
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

    fn vol(mount: &str, total: u64, available: u64) -> Volume {
        Volume {
            mount_point: PathBuf::from(mount),
            total,
            available,
        }
    }

    #[test]
    fn resolve_picks_longest_matching_mount_point() {
        let volumes = vec![vol("/", 500, 100), vol("/Volumes/Data", 1000, 400)];
        // /Volumes/Data/projects は "/" と "/Volumes/Data" の両方にマッチするが、
        // より長い "/Volumes/Data" を選ぶ。used = 1000 - 400 = 600。
        let usage = resolve(Path::new("/Volumes/Data/projects"), &volumes);
        assert_eq!(
            usage,
            Some(DiskUsage {
                used: 600,
                total: 1000
            })
        );
    }

    #[test]
    fn resolve_matches_on_path_components_not_string_prefix() {
        // "/foobar" は "/foo" のマウント配下ではない（文字列前方一致では誤って一致する）。
        // ルート "/" にはマッチするので、ルートの使用量が返る。
        let volumes = vec![vol("/", 500, 100), vol("/foo", 1000, 400)];
        let usage = resolve(Path::new("/foobar"), &volumes);
        assert_eq!(
            usage,
            Some(DiskUsage {
                used: 400,
                total: 500
            })
        );
    }

    #[test]
    fn resolve_returns_none_when_no_mount_point_matches() {
        // ルートを含まないボリューム集合で、どのマウントポイントにも属さない dir。
        let volumes = vec![vol("/Volumes/Data", 1000, 400)];
        assert_eq!(resolve(Path::new("/etc"), &volumes), None);
    }

    #[test]
    fn percent_is_used_over_total_rounded() {
        let usage = DiskUsage {
            used: 120,
            total: 500,
        };
        assert_eq!(usage.percent(), 24);
    }

    const GIB: u64 = 1024 * 1024 * 1024;

    #[test]
    fn format_disk_field_shows_used_total_gib_and_percent() {
        let usage = DiskUsage {
            used: 120 * GIB,
            total: 500 * GIB,
        };
        assert_eq!(format_disk_field(Some(usage)), "Disk 120.0/500.0G (24%)");
    }

    #[test]
    fn format_disk_field_shows_placeholder_when_volume_unresolved() {
        assert_eq!(format_disk_field(None), "Disk n/a");
    }

    #[test]
    fn format_disk_field_scales_to_terabytes() {
        // total が 1024GiB 以上のとき T 単位へスケールする。
        let usage = DiskUsage {
            used: 1024 * GIB,      // 1.0T
            total: 2 * 1024 * GIB, // 2.0T
        };
        assert_eq!(format_disk_field(Some(usage)), "Disk 1.0/2.0T (50%)");
    }
}
