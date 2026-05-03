use ratatui::widgets::TableState;

#[derive(Debug)]
pub struct BookmarkState {
    pub table_state: TableState,
    pub paths: Vec<String>,
}

impl BookmarkState {
    pub fn new(paths: Vec<String>) -> Self {
        Self {
            table_state: TableState::default(),
            paths,
        }
    }

    pub fn next(&mut self) {}

    pub fn prev(&mut self) {}

    pub fn selected_path(&self) -> Option<&str> {
        Some("")
    }
}
