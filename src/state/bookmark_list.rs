use crate::bookmark;
use crate::state::cursor;
use ratatui::widgets::TableState;
use std::collections::HashSet;

#[derive(Debug)]
pub struct BookmarkListState {
    pub paths: Vec<String>,
    pub table_state: TableState,
}

impl BookmarkListState {
    pub fn new(bookmarked_paths: &HashSet<String>) -> Self {
        let paths = bookmark::sorted_paths(bookmarked_paths);
        let mut table_state = TableState::default();
        if !paths.is_empty() {
            table_state.select(Some(0));
        }
        Self { paths, table_state }
    }

    pub fn next(&mut self) {
        cursor::move_next(&mut self.table_state, self.paths.len());
    }

    pub fn prev(&mut self) {
        cursor::move_prev(&mut self.table_state, self.paths.len());
    }

    pub fn selected_path(&self) -> Option<&str> {
        self.table_state
            .selected()
            .and_then(|i| self.paths.get(i))
            .map(|s| s.as_str())
    }
}
