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

    pub fn formatted_size(&self) -> String {
        const KB: u64 = 1024;
        const MB: u64 = 1024 * KB;
        const GB: u64 = 1024 * MB;

        let bytes = self.file_size();
        let formatted = bytes.to_formatted_string(&Locale::en);
        if bytes >= GB {
            format!("{formatted} bytes ({:.1} GB)", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{formatted} bytes ({:.1} MB)", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{formatted} bytes ({:.1} KB)", bytes as f64 / KB as f64)
        } else {
            format!("{formatted} bytes")
        }
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
