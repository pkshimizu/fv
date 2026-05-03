use ratatui::widgets::TableState;

#[derive(Debug)]
pub struct BookmarkState {
    pub table_state: TableState,
    pub paths: Vec<String>,
}

impl BookmarkState {
    pub fn new(paths: Vec<String>) -> Self {
        let mut table_state = TableState::default();
        if !paths.is_empty() {
            table_state.select(Some(0));
        }
        Self { table_state, paths }
    }

    pub fn next(&mut self) {
        if self.paths.is_empty() {
            return;
        }
        if let Some(selected) = self.table_state.selected() {
            if selected < self.paths.len() - 1 {
                self.table_state.select(Some(selected + 1));
            }
        }
    }

    pub fn prev(&mut self) {
        if self.paths.is_empty() {
            return;
        }
        if let Some(selected) = self.table_state.selected() {
            if selected > 0 {
                self.table_state.select(Some(selected - 1));
            }
        }
    }

    pub fn first(&mut self) {
        if self.paths.is_empty() {
            return;
        }
        self.table_state.select(Some(0));
    }

    pub fn last(&mut self) {
        if self.paths.is_empty() {
            return;
        }
        self.table_state.select(Some(self.paths.len() - 1));
    }

    pub fn selected_path(&self) -> Option<&str> {
        let selected_index = self.table_state.selected();
        selected_index.and_then(|i| self.paths.get(i).map(String::as_str))
    }

    pub fn remove(&mut self, path: &str) {
        self.paths.retain(|p| p != path);
        if let Some(selected) = self.table_state.selected() {
            if self.paths.is_empty() {
                self.table_state.select(None);
            } else if selected >= self.paths.len() {
                self.table_state.select(Some(self.paths.len() - 1));
            }
        }
    }
}
