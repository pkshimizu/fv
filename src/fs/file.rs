use std::fs::read_dir;

#[derive(Debug)]
pub struct File {
    pub path: String,
}

impl File {
    pub fn new(path: String) -> File {
        Self { path }
    }

    pub fn list(self) -> Vec<File> {
        let result = read_dir(self.path.to_string());
        if result.is_ok() {
            let entries = result.unwrap().collect::<Vec<_>>();
            let mut files: Vec<File> = Vec::new();
            for entry in entries {
                if entry.is_ok() {
                    let path = entry.unwrap().path();
                    files.push(File::new(path.to_str().unwrap().to_string()));
                }
            }
            return files;
        }
        Vec::new()
    }

    pub fn size(self) -> u64 {
        std::fs::metadata(self.path).unwrap().len()
    }

    pub fn is_file(self) -> bool {
        std::fs::metadata(self.path.clone()).unwrap().is_file()
    }

    pub fn is_dir(self) -> bool {
        std::fs::metadata(self.path.clone()).unwrap().is_dir()
    }
}
