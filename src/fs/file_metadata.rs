use crate::fs::VFileTime;
use crate::fs::permissions::VPermissions;
use std::fs::Metadata;
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

    pub fn mode(&self) -> u32 {
        self.metadata.mode()
    }

    pub fn uid(&self) -> u32 {
        self.metadata.uid()
    }

    pub fn gid(&self) -> u32 {
        self.metadata.gid()
    }

    pub fn nlink(&self) -> u64 {
        self.metadata.nlink()
    }

    pub fn ino(&self) -> u64 {
        self.metadata.ino()
    }

    pub fn dev(&self) -> u64 {
        self.metadata.dev()
    }

    pub fn blksize(&self) -> u64 {
        self.metadata.blksize()
    }

    pub fn blocks(&self) -> u64 {
        self.metadata.blocks()
    }
}
