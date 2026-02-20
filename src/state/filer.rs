use std::path::PathBuf;

#[derive(Debug)]
pub struct FilerState {
    pub current_dir_path: PathBuf,
}

impl FilerState {
    pub fn new() -> Self {
        Self {
            current_dir_path: dirs::home_dir().unwrap(),
        }
    }
}
