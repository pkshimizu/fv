use crate::fs::VFileTime;
use crate::fs::permissions::VPermissions;
use std::fs::Metadata;

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

    pub fn modified(&self) -> anyhow::Result<VFileTime> {
        let modified = self.metadata.modified()?;
        Ok(VFileTime::new(modified))
    }

    pub fn permissions(&self) -> VPermissions {
        VPermissions::new(self.metadata.permissions())
    }
}
