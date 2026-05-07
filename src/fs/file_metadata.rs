use crate::fs::VFileTime;
use crate::fs::permissions::VPermissions;
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

    pub fn file_type(&self) -> String {
        if self.is_symlink() {
            "Symlink".to_string()
        } else if self.is_dir() {
            "Directory".to_string()
        } else if self.is_file() {
            "File".to_string()
        } else {
            "Other".to_string()
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
