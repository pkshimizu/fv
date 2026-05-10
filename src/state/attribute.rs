use crate::fs::VFile;
use crate::fs::VFileMetadata;
use crate::state::table_cursor::TableCursor;
use anyhow::Result;
use ratatui::widgets::TableState;

#[derive(Debug)]
pub struct AttributeState {
    pub table_state: TableState,
    pub file_name: String,
    pub entries: Vec<(&'static str, String)>,
}

impl AttributeState {
    pub fn new(file: &VFile) -> Result<Self> {
        let metadata = file.metadata()?;
        let file_name = file.file_name().unwrap_or("(unknown)").to_string();
        let entries = Self::build_entries(metadata);

        let mut table_state = TableState::default();
        table_state.select(Some(0));

        Ok(Self {
            table_state,
            file_name,
            entries,
        })
    }

    fn build_entries(metadata: &VFileMetadata) -> Vec<(&'static str, String)> {
        let mut entries = Vec::new();
        entries.extend([
            ("File Type", metadata.file_type().to_string()),
            ("Size", metadata.formatted_size()),
            ("Permissions", metadata.permissions().to_rwx_string()),
        ]);

        #[cfg(unix)]
        entries.extend([
            ("Mode", format!("{:04o}", metadata.mode() & 0o7777)),
            ("Owner (UID)", metadata.uid().to_string()),
            ("Group (GID)", metadata.gid().to_string()),
            ("Hard Links", metadata.nlink().to_string()),
            ("Inode", metadata.ino().to_string()),
            ("Device ID", metadata.dev().to_string()),
            ("Block Size", metadata.blksize().to_string()),
            ("Blocks", metadata.blocks().to_string()),
        ]);

        entries.extend([
            (
                "Created",
                metadata
                    .created()
                    .map(|t| t.to_string())
                    .unwrap_or_else(|_| "-".to_string()),
            ),
            (
                "Accessed",
                metadata
                    .accessed()
                    .map(|t| t.to_string())
                    .unwrap_or_else(|_| "-".to_string()),
            ),
            (
                "Modified",
                metadata
                    .modified()
                    .map(|t| t.to_string())
                    .unwrap_or_else(|_| "-".to_string()),
            ),
        ]);
        entries
    }

    fn cursor(&mut self) -> TableCursor {
        TableCursor::new(&mut self.table_state, self.entries.len())
    }

    pub fn next(&mut self) {
        self.cursor().next();
    }

    pub fn prev(&mut self) {
        self.cursor().prev();
    }
}
