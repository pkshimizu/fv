use crate::fs::file_time::VFileTime;
use crate::fs::permissions::VPermissions;
use anyhow::Result;
use std::fs::read_dir;
use std::path::Path;

#[derive(Debug)]
pub struct VFile {
    pub path: String,
}

impl VFile {
    pub fn new(path: String) -> Self {
        Self { path }
    }

    pub fn absolute_path(&self) -> String {
        self.path.clone()
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
        let entries = result.collect::<Vec<_>>();
        let mut files: Vec<VFile> = Vec::new();
        for entry in entries {
            let path = entry?.path();
            if let Some(path_str) = path.to_str() {
                files.push(VFile::new(path_str.to_string()));
            }
        }
        Ok(files)
    }

    pub fn file_size(&self) -> Result<u64> {
        let result = std::fs::metadata(&self.path)?;
        Ok(result.len())
    }

    pub fn is_dir(&self) -> Result<bool> {
        let result = std::fs::metadata(&self.path)?;
        Ok(result.is_dir())
    }

    pub fn modified(&self) -> Result<VFileTime> {
        let result = std::fs::metadata(&self.path)?;
        let modified = result.modified()?;
        Ok(VFileTime::new(modified))
    }

    pub fn permissions(&self) -> Result<VPermissions> {
        let result = std::fs::metadata(&self.path)?;
        Ok(VPermissions::new(result.permissions()))
    }
}
