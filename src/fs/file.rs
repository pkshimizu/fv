use crate::fs::file_metadata::VFileMetadata;
use anyhow::{Context, Result};
use std::fs::{create_dir, read_dir};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct VFile {
    path: String,
    metadata: Option<VFileMetadata>,
}

// VFileMetadataのMetadataがPartialEqを実装していないため、pathのみでeqを実装
impl PartialEq for VFile {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Eq for VFile {}

impl VFile {
    pub fn new(path: impl Into<String>) -> Self {
        let path = path.into();
        let metadata = std::fs::metadata(&path).ok().map(VFileMetadata::new);
        Self { path, metadata }
    }

    pub fn absolute_path(&self) -> &str {
        &self.path
    }

    pub fn file_name(&self) -> Option<String> {
        let file_name = Path::new(&self.path).file_name()?;
        let file_name_str = file_name.to_str()?;
        Some(file_name_str.to_string())
    }

    pub fn parent_dir(&self) -> Option<VFile> {
        let path = Path::new(&self.path);
        let parent = path.parent()?;
        let parent_path = parent.to_str()?;
        Some(VFile::new(parent_path))
    }

    pub fn list(&self) -> Result<Vec<VFile>> {
        let result = read_dir(&self.path)?;
        let mut files: Vec<VFile> = Vec::new();
        for entry in result {
            let path = entry?.path();
            if let Some(path_str) = path.to_str() {
                files.push(VFile::new(path_str));
            }
        }
        Ok(files)
    }

    pub fn metadata(&self) -> Result<&VFileMetadata> {
        self.metadata
            .as_ref()
            .with_context(|| format!("{}: No metadata", self.path))
    }

    pub fn is_dir(&self) -> Result<bool> {
        Ok(self.metadata()?.is_dir())
    }

    pub fn create_dir(&self, dir_name: &str) -> Result<()> {
        let path = Path::new(self.absolute_path());
        let dir_path = path.join(dir_name);
        create_dir(&dir_path)
            .with_context(|| format!("{}: Failed to create directory", dir_path.display()))?;

        Ok(())
    }

    pub fn delete(&self) -> Result<()> {
        let path = self.absolute_path();
        trash::delete(path).with_context(|| format!("{}: Failed to trash", self.path))?;

        Ok(())
    }
}
