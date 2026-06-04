//! カレントディレクトリが属するストレージボリュームの使用状況（Disk Usage）。
//! sysinfo 依存の I/O（`DiskUsageReader`）と、ディレクトリ→ボリューム照合・整形の
//! 純粋ロジックを分離し、後者をユニットテスト可能にする。

use crate::os::{REFRESH_INTERVAL, RefreshThrottle, format_used_total};
use std::path::{Path, PathBuf};
use sysinfo::Disks;

/// マウント済みボリュームの容量情報（sysinfo の 1 ディスク分に相当）。
pub(crate) struct Volume {
    pub(crate) mount_point: PathBuf,
    pub(crate) total: u64,
    pub(crate) available: u64,
}

/// あるディレクトリが属するボリュームの使用量と総容量。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DiskUsage {
    pub(crate) used: u64,
    pub(crate) total: u64,
}

impl DiskUsage {
    /// 使用率（%）を四捨五入で返す。total が 0 のときは 0、上限は 100 にクランプする
    /// （sysinfo が一時的に available > total を返す等の異常値で 100 を超えないように）。
    pub(crate) fn percent(self) -> u64 {
        if self.total == 0 {
            return 0;
        }
        ((self.used as f64 / self.total as f64 * 100.0).round() as u64).min(100)
    }
}

/// `dir` を含むボリュームをマウントポイントの最長プレフィックス一致で解決する。
/// `used = total - available`。一致するボリュームが無ければ `None`。
pub(crate) fn resolve(dir: &Path, volumes: &[Volume]) -> Option<DiskUsage> {
    volumes
        .iter()
        .filter(|v| dir.starts_with(&v.mount_point))
        .max_by_key(|v| v.mount_point.as_os_str().len())
        .map(|v| DiskUsage {
            used: v.total.saturating_sub(v.available),
            total: v.total,
        })
}

/// sysinfo の `Disks` を保持し、約5秒ごとに容量を再取得する。
/// 呼び出し側（ヘッダー）は sysinfo を直接触らず、本 reader 越しにカレントディレクトリの
/// `DiskUsage` を読む。System Info reader と同型。
pub(crate) struct DiskUsageReader {
    disks: Disks,
    /// `disks` から変換済みのボリューム一覧。`tick()` のリフレッシュ時にのみ再構築し、
    /// 毎フレーム呼ばれる `usage_for` ではこれを参照するだけにする（確保を約5秒に1回へ）。
    volumes: Vec<Volume>,
    throttle: RefreshThrottle,
}

impl DiskUsageReader {
    pub(crate) fn new() -> Self {
        let disks = Disks::new_with_refreshed_list();
        let volumes = collect_volumes(&disks);
        Self {
            disks,
            volumes,
            throttle: RefreshThrottle::new(REFRESH_INTERVAL),
        }
    }

    /// メインループの tick ごとに呼ぶ。スロットルが許せばボリューム一覧の容量を再取得する。
    pub(crate) fn tick(&mut self) {
        if self.throttle.tick() {
            self.disks.refresh(true);
            self.volumes = collect_volumes(&self.disks);
        }
    }

    /// `dir` が属するボリュームの使用状況を返す。特定できなければ `None`。
    pub(crate) fn usage_for(&self, dir: &Path) -> Option<DiskUsage> {
        resolve(dir, &self.volumes)
    }
}

/// sysinfo の `Disks` から `Volume` 一覧へ変換する。`tick()` のリフレッシュ時のみ呼ぶ。
fn collect_volumes(disks: &Disks) -> Vec<Volume> {
    disks
        .list()
        .iter()
        .map(|d| Volume {
            mount_point: d.mount_point().to_path_buf(),
            total: d.total_space(),
            available: d.available_space(),
        })
        .collect()
}

/// ヘッダーに載せる Disk フィールド文字列を整形する。
/// 使用量/総容量(使用率) 形式。例: `Disk 120.0/500.0G (24%)`。
/// ボリューム未特定（`None`）のときは `Disk n/a`。
pub(crate) fn format_disk_field(usage: Option<DiskUsage>) -> String {
    match usage {
        Some(u) => format!(
            "Disk {} ({}%)",
            format_used_total(u.used, u.total),
            u.percent()
        ),
        None => "Disk n/a".to_string(),
    }
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

    #[test]
    fn percent_clamps_to_100_when_used_exceeds_total() {
        // sysinfo が一時的に available > total を返す等の異常値でも 100 を超えない。
        let usage = DiskUsage {
            used: 600,
            total: 500,
        };
        assert_eq!(usage.percent(), 100);
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
