use crate::fs::file_metadata::VFileMetadata;
use anyhow::{Context, Result};
use std::fs::read_dir;
use std::path::Path;

#[derive(Debug)]
pub struct VFile {
    path: String,
    metadata: Option<VFileMetadata>,
}

impl VFile {
    pub fn new(path: String) -> Self {
        let metadata = std::fs::metadata(&path).ok().map(VFileMetadata::new);
        Self { path, metadata }
    }

    pub fn absolute_path(&self) -> String {
        self.path.to_string()
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
        Some(VFile::new(parent_path.to_string()))
    }

    pub fn list(&self) -> Result<Vec<VFile>> {
        let result = read_dir(&self.path)?;
        let mut files: Vec<VFile> = Vec::new();
        for entry in result {
            let path = entry?.path();
            if let Some(path_str) = path.to_str() {
                files.push(VFile::new(path_str.to_string()));
            }
        }
        Ok(files)
    }

    pub fn metadata(&self) -> Result<&VFileMetadata> {
        self.metadata.as_ref().context("no metadata")
    }
}
