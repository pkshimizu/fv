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

    pub fn file_name(&self) -> String {
        if let Some(file_name) = Path::new(&self.path).file_name() {
            if let Some(file_name_str) = file_name.to_str() {
                return file_name_str.to_string();
            }
        }
        "".to_string()
    }

    pub fn parent_dir(&self) -> VFile {
        let path = Path::new(&self.path);
        if let Some(parent) = path.parent() {
            if let Some(parent_path) = parent.to_str() {
                return VFile::new(parent_path.to_string());
            }
        }
        VFile::new(self.path.to_string())
    }

    pub fn list(&self) -> Vec<VFile> {
        let result = read_dir(&self.path);
        if let Ok(result) = result {
            let entries = result.collect::<Vec<_>>();
            let mut files: Vec<VFile> = Vec::new();
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if let Some(path_str) = path.to_str() {
                        files.push(VFile::new(path_str.to_string()));
                    }
                }
            }
            return files;
        }
        Vec::new()
    }

    pub fn list_size(&self) -> usize {
        let result = read_dir(&self.path);
        if let Ok(result) = result {
            return result.count();
        }
        0
    }

    pub fn file_size(&self) -> u64 {
        let result = std::fs::metadata(&self.path);
        if let Ok(result) = result {
            return result.len();
        }
        0
    }

    pub fn is_dir(&self) -> bool {
        let result = std::fs::metadata(&self.path);
        if let Ok(result) = result {
            return result.is_dir();
        }
        false
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
