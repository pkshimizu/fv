use ratatui::widgets::TableState;
use std::path::PathBuf;

#[derive(Debug)]
pub struct FilerState {
    pub current_dir_path: PathBuf,
    pub file_table_state: TableState,
}

impl FilerState {
    pub fn new() -> Self {
        Self {
            current_dir_path: dirs::home_dir().unwrap(),
            file_table_state: TableState::default(),
        }
    }
}
