use crate::fs::VFile;
use ratatui::widgets::TableState;

#[derive(Debug)]
pub struct FilerState {
    pub current_dir: VFile,
    pub current_dir_files: Vec<VFile>,
    pub file_table_state: TableState,
}

impl FilerState {
    pub fn new() -> Self {
        let home_dir = dirs::home_dir().unwrap();
        let current_dir_path = home_dir.to_str().unwrap();
        let current_dir = VFile::new(current_dir_path.to_string());
        let current_dir_files = current_dir.list();

        let mut state = Self {
            current_dir,
            current_dir_files,
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

    pub fn change_dir_in_select_dir(&mut self) {
        if let Some(selected) = self.file_table_state.selected() {
            let selected_file_path = self.current_dir_files[selected].absolute_path();
            let selected_file = VFile::new(selected_file_path.to_string());
            if selected_file.is_dir() {
                self.current_dir = selected_file;
                self.current_dir_files = self.current_dir.list();
                self.file_table_state.select(Some(0));
            }
        }
    }

    pub fn change_dir_in_parent_dir(&mut self) {
        let parent_dir = self.current_dir.parent_dir();
        self.current_dir = parent_dir;
        self.current_dir_files = self.current_dir.list();
        self.file_table_state.select(Some(0));
    }

    pub fn refresh_files(&mut self) {
        let selected_index = self.file_table_state.selected();
        let selected_name = selected_index
            .and_then(|i| self.current_dir_files.get(i))
            .map(|f| f.file_name());

        self.current_dir_files = self.current_dir.list();

        if let Some(name) = selected_name {
            let new_index = self
                .current_dir_files
                .iter()
                .position(|f| f.file_name() == name)
                .unwrap_or(0);
            self.file_table_state.select(Some(
                new_index.min(self.current_dir_files.len().saturating_sub(1)),
            ));
        } else {
            self.file_table_state.select(Some(0));
        }
    }
}
