use crate::fs::VFile;
use crate::fs::VFileMetadata;
use crate::state::table_cursor::TableCursor;
use anyhow::Result;
use num_format::{Locale, ToFormattedString};
use ratatui::widgets::TableState;

#[derive(Debug)]
pub struct AttributeState {
    pub table_state: TableState,
    pub metadata: VFileMetadata,
    pub file_name: String,
    row_count: usize,
}

impl AttributeState {
    pub fn new(file: &VFile) -> Result<Self> {
        let metadata = file.metadata()?.clone();
        let file_name = file.file_name().unwrap_or("(unknown)").to_string();

        let mut table_state = TableState::default();
        table_state.select(Some(0));

        Ok(Self {
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

    pub fn entries(&self) -> Vec<(&'static str, String)> {
        let mut entries = Vec::new();
        entries.extend([
            ("File Type", self.metadata.file_type()),
            (
                "Size",
                format!(
                    "{} bytes",
                    self.metadata.file_size().to_formatted_string(&Locale::en)
                ),
            ),
            ("Permissions", self.metadata.permissions().to_rwx_string()),
        ]);

        #[cfg(unix)]
        entries.extend([
            ("Mode", format!("{:04o}", self.metadata.mode() & 0o7777)),
            ("Owner (UID)", self.metadata.uid().to_string()),
            ("Group (GID)", self.metadata.gid().to_string()),
            ("Hard Links", self.metadata.nlink().to_string()),
            ("Inode", self.metadata.ino().to_string()),
            ("Device ID", self.metadata.dev().to_string()),
            ("Block Size", self.metadata.blksize().to_string()),
            ("Blocks", self.metadata.blocks().to_string()),
        ]);

        entries.extend([
            (
                "Created",
                self.metadata
                    .created()
                    .map(|t| t.to_string())
                    .unwrap_or_else(|_| "-".to_string()),
            ),
            (
                "Accessed",
                self.metadata
                    .accessed()
                    .map(|t| t.to_string())
                    .unwrap_or_else(|_| "-".to_string()),
            ),
            (
                "Modified",
                self.metadata
                    .modified()
                    .map(|t| t.to_string())
                    .unwrap_or_else(|_| "-".to_string()),
            ),
        ]);
        entries
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
