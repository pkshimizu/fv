use ratatui::widgets::TableState;
use std::fs;
use std::fs::DirEntry;

#[derive(Debug)]
pub struct FilerState {
    pub current_dir_path: String,
    pub current_dir_files: Vec<std::io::Result<DirEntry>>,
    pub file_table_state: TableState,
}

impl FilerState {
    pub fn new() -> Self {
        let home_dir = dirs::home_dir().unwrap();
        let current_dir_path = home_dir.to_str().unwrap();
        let files = fs::read_dir(current_dir_path).unwrap().collect::<Vec<_>>();

        let mut state = Self {
            current_dir_path: current_dir_path.to_string(),
            current_dir_files: files,
            file_table_state: TableState::default(),
        };
        state.file_table_state.select(Some(0));
        state
    }

    pub fn next(&mut self) {
        if let Some(selected) = self.file_table_state.selected() {
            if selected < self.current_dir_files.len() - 1 {
                self.file_table_state.select(Some(selected + 1));
            }
        }
    }

    pub fn prev(&mut self) {
        if let Some(selected) = self.file_table_state.selected() {
            if selected > 0 {
                self.file_table_state.select(Some(selected - 1));
            }
        }
    }

    pub fn first(&mut self) {
        self.file_table_state.select(Some(0));
    }

    pub fn last(&mut self) {
        self.file_table_state
            .select(Some(self.current_dir_files.len() - 1));
    }
}
