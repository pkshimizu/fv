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
        Path::new(&self.path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    }

    pub fn parent_dir(&self) -> VFile {
        let path = Path::new(&self.path);
        VFile::new(path.parent().unwrap().to_str().unwrap().to_string())
    }

    pub fn list(&self) -> Vec<VFile> {
        let result = read_dir(&self.path);
        if result.is_ok() {
            let entries = result.unwrap().collect::<Vec<_>>();
            let mut files: Vec<VFile> = Vec::new();
            for entry in entries {
                if entry.is_ok() {
                    let path = entry.unwrap().path();
                    files.push(VFile::new(path.to_str().unwrap().to_string()));
                }
            }
            return files;
        }
        Vec::new()
    }

    pub fn list_size(&self) -> usize {
        let result = read_dir(&self.path);
        if result.is_ok() {
            return result.unwrap().count();
        }
        0
    }

    pub fn file_size(&self) -> u64 {
        let result = std::fs::metadata(&self.path);
        if result.is_ok() {
            return result.unwrap().len();
        }
        0
    }

    pub fn is_dir(&self) -> bool {
        let result = std::fs::metadata(&self.path);
        if result.is_ok() {
            return result.unwrap().is_dir();
        }
        false
    }
}
