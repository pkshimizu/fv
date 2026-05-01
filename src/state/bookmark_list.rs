use ratatui::widgets::TableState;
use std::collections::HashSet;

#[derive(Debug)]
pub struct BookmarkListState {
    pub paths: Vec<String>,
    pub table_state: TableState,
}

impl BookmarkListState {
    pub fn new(bookmarked_paths: &HashSet<String>) -> Self {
        let mut paths: Vec<String> = bookmarked_paths.iter().cloned().collect();
        paths.sort();
        let mut table_state = TableState::default();
        if !paths.is_empty() {
            table_state.select(Some(0));
        }
        Self { paths, table_state }
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
}
