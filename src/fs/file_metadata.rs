use crate::fs::VFileTime;
use crate::fs::permissions::VPermissions;
use num_format::{Locale, ToFormattedString};
use std::fs::Metadata;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[derive(Debug, Clone)]
pub struct VFileMetadata {
    metadata: Metadata,
}

impl VFileMetadata {
    pub fn new(metadata: Metadata) -> Self {
        Self { metadata }
    }

    pub fn file_size(&self) -> u64 {
        self.metadata.len()
    }

    /// Filer 一覧向けのコンパクトなサイズ表記。例: `234.5 MB` / `512 B`。
    /// 1024 以上は単位（KB/MB/GB/TB）付き小数1桁、未満は整数 + `B`。
    pub fn compact_size(&self) -> String {
        compact_size_str(self.file_size())
    }

    /// 属性 / info パネル向けの詳細なサイズ表記。
    /// 例: `123.4 MB (123,400,000 bytes)`。1024 未満は `512 bytes` のみ。
    pub fn formatted_size(&self) -> String {
        verbose_size_str(self.file_size())
    }

    pub fn file_type(&self) -> &str {
        if self.is_symlink() {
            "Symlink"
        } else if self.is_dir() {
            "Directory"
        } else if self.is_file() {
            "File"
        } else {
            "Other"
        }
    }

    pub fn is_dir(&self) -> bool {
        self.metadata.is_dir()
    }

    pub fn is_file(&self) -> bool {
        self.metadata.is_file()
    }

    pub fn is_symlink(&self) -> bool {
        self.metadata.is_symlink()
    }

    pub fn modified(&self) -> anyhow::Result<VFileTime> {
        let modified = self.metadata.modified()?;
        Ok(VFileTime::new(modified))
    }

    pub fn accessed(&self) -> anyhow::Result<VFileTime> {
        let accessed = self.metadata.accessed()?;
        Ok(VFileTime::new(accessed))
    }

    pub fn created(&self) -> anyhow::Result<VFileTime> {
        let created = self.metadata.created()?;
        Ok(VFileTime::new(created))
    }

    pub fn permissions(&self) -> VPermissions {
        VPermissions::new(self.metadata.permissions())
    }

    #[cfg(unix)]
    pub fn mode(&self) -> u32 {
        self.metadata.mode()
    }

    #[cfg(unix)]
    pub fn uid(&self) -> u32 {
        self.metadata.uid()
    }

    #[cfg(unix)]
    pub fn gid(&self) -> u32 {
        self.metadata.gid()
    }

    #[cfg(unix)]
    pub fn nlink(&self) -> u64 {
        self.metadata.nlink()
    }

    #[cfg(unix)]
    pub fn ino(&self) -> u64 {
        self.metadata.ino()
    }

    #[cfg(unix)]
    pub fn dev(&self) -> u64 {
        self.metadata.dev()
    }

    #[cfg(unix)]
    pub fn blksize(&self) -> u64 {
        self.metadata.blksize()
    }

    #[cfg(unix)]
    pub fn blocks(&self) -> u64 {
        self.metadata.blocks()
    }
}

const KB: u64 = 1024;
const MB: u64 = 1024 * KB;
const GB: u64 = 1024 * MB;
const TB: u64 = 1024 * GB;

/// 1024 以上のバイト数を単位付き小数1桁の文字列（例: `1.2 GB`）に変換する。
/// 1024 未満は単位の梯子に乗らないため `None` を返す。基数は 1024、
/// 最上位は TB（実在し得る単一ファイルを十分カバーする）。
/// compact_size と formatted_size の双方がこの梯子を共有する。
fn unit_size(bytes: u64) -> Option<String> {
    if bytes >= TB {
        Some(format!("{:.1} TB", bytes as f64 / TB as f64))
    } else if bytes >= GB {
        Some(format!("{:.1} GB", bytes as f64 / GB as f64))
    } else if bytes >= MB {
        Some(format!("{:.1} MB", bytes as f64 / MB as f64))
    } else if bytes >= KB {
        Some(format!("{:.1} KB", bytes as f64 / KB as f64))
    } else {
        None
    }
}

fn compact_size_str(bytes: u64) -> String {
    unit_size(bytes).unwrap_or_else(|| format!("{bytes} B"))
}

fn verbose_size_str(bytes: u64) -> String {
    let separated = bytes.to_formatted_string(&Locale::en);
    match unit_size(bytes) {
        Some(unit) => format!("{unit} ({separated} bytes)"),
        None => format!("{separated} bytes"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_size_is_none_below_one_kilobyte() {
        assert_eq!(unit_size(0), None);
        assert_eq!(unit_size(1023), None);
    }

    #[test]
    fn unit_size_climbs_the_ladder_at_each_threshold() {
        assert_eq!(unit_size(1024).as_deref(), Some("1.0 KB"));
        assert_eq!(unit_size(MB).as_deref(), Some("1.0 MB"));
        assert_eq!(unit_size(GB).as_deref(), Some("1.0 GB"));
        assert_eq!(unit_size(TB).as_deref(), Some("1.0 TB"));
    }

    #[test]
    fn unit_size_extends_beyond_gigabytes_to_terabytes() {
        // 旧実装は GB 止まりで 2 TB を "2048.0 GB" と表示していた。
        assert_eq!(unit_size(2 * TB).as_deref(), Some("2.0 TB"));
    }

    #[test]
    fn compact_size_uses_bare_byte_count_below_one_kilobyte() {
        assert_eq!(compact_size_str(0), "0 B");
        assert_eq!(compact_size_str(512), "512 B");
        assert_eq!(compact_size_str(1023), "1023 B");
    }

    #[test]
    fn compact_size_uses_units_at_and_above_one_kilobyte() {
        assert_eq!(compact_size_str(1024), "1.0 KB");
        assert_eq!(compact_size_str(5 * MB), "5.0 MB");
        assert_eq!(compact_size_str(2 * TB), "2.0 TB");
    }

    #[test]
    fn verbose_size_omits_parentheses_below_one_kilobyte() {
        assert_eq!(verbose_size_str(512), "512 bytes");
        // 区切りは 1000 以上で入る。
        assert_eq!(verbose_size_str(1023), "1,023 bytes");
    }

    #[test]
    fn verbose_size_leads_with_unit_then_raw_bytes() {
        assert_eq!(verbose_size_str(1024), "1.0 KB (1,024 bytes)");
        assert_eq!(
            verbose_size_str(1_288_490_189),
            "1.2 GB (1,288,490,189 bytes)"
        );
    }
}
