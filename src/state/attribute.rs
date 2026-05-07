use crate::fs::VFile;
use crate::fs::VFileMetadata;
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
            row_count: Self::row_count(),
        })
    }

    pub fn row_count() -> usize {
        let base = 3; // File Type, Size, Permissions
        let timestamps = 3; // Created, Accessed, Modified
        #[cfg(unix)]
        let unix_fields = 8; // Mode, UID, GID, Hard Links, Inode, Device ID, Block Size, Blocks
        #[cfg(not(unix))]
        let unix_fields = 0;
        base + unix_fields + timestamps
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
