use crate::state::table_cursor::TableCursor;
use ratatui::widgets::TableState;

#[derive(Debug)]
pub struct GrepState {
    pub table_state: TableState,
    pub paths: Vec<String>,
}

impl GrepState {
    pub fn new(paths: Vec<String>) -> Self {
        let mut table_state = TableState::default();
        if !paths.is_empty() {
            table_state.select(Some(0));
        }
        Self { table_state, paths }
    }

    fn cursor(&mut self) -> TableCursor {
        TableCursor::new(&mut self.table_state, self.paths.len())
    }

    pub fn next(&mut self) {
        self.cursor().next();
    }

    pub fn prev(&mut self) {
        self.cursor().prev();
    }

    pub fn first(&mut self) {
        self.cursor().first();
    }

    pub fn last(&mut self) {
        self.cursor().last();
    }

    pub fn selected_path(&self) -> Option<&str> {
        self.table_state
            .selected()
            .and_then(|i| self.paths.get(i).map(String::as_str))
    }
}
