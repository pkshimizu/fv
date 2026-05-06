use crate::fs::{VFile, VFileMetadata};
use crate::state::table_cursor::TableCursor;
use ratatui::widgets::TableState;

const ATTRIBUTE_COUNT: usize = 13;

#[derive(Debug)]
pub struct AttributeState {
    pub table_state: TableState,
    pub metadata: VFileMetadata,
    pub file_name: String,
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
        })
    }

    fn cursor(&mut self) -> TableCursor {
        TableCursor::new(&mut self.table_state, ATTRIBUTE_COUNT)
    }

    pub fn next(&mut self) {
        self.cursor().next();
    }

    pub fn prev(&mut self) {
        self.cursor().prev();
    }
}
