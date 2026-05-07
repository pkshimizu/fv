use crate::fs::{VFile, VFileMetadata};
use crate::state::table_cursor::TableCursor;
use ratatui::widgets::TableState;

#[derive(Debug)]
pub struct AttributeState {
    pub table_state: TableState,
    pub metadata: VFileMetadata,
    pub file_name: String,
    row_count: usize,
}

impl AttributeState {
    pub fn new(file: &VFile) -> Option<Self> {
        let metadata = file.metadata().ok()?.clone();
        let file_name = file.file_name().unwrap_or("(unknown)").to_string();

        let mut table_state = TableState::default();
        table_state.select(Some(0));

        Some(Self {
            table_state,
            metadata,
            file_name,
            row_count: 0,
        })
    }

    pub fn set_row_count(&mut self, count: usize) {
        self.row_count = count;
    }

    fn cursor(&mut self) -> TableCursor {
        TableCursor::new(&mut self.table_state, self.row_count)
    }

    pub fn next(&mut self) {
        self.cursor().next();
    }

    pub fn prev(&mut self) {
        self.cursor().prev();
    }
}
