use crate::component::{Action, Component};
use crate::fs::VFile;
use crate::fs::VFileMetadata;
use crate::state::table_cursor::TableCursor;
use crate::ui::widgets::build_focused_block;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Cell, Row, Table, TableState};

pub struct AttributeComponent {
    table_state: TableState,
    file_name: String,
    entries: Vec<(&'static str, String)>,
}

impl AttributeComponent {
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
            ("Size", metadata.verbose_size()),
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

    fn cursor(&mut self) -> TableCursor<'_> {
        TableCursor::new(&mut self.table_state, self.entries.len())
    }
}

impl Component for AttributeComponent {
    fn keymap(&self) -> &'static str {
        "↑↓: Move  a/Esc: Close"
    }

    fn handle_event(&mut self, event: KeyEvent) -> Result<Action> {
        match event.code {
            KeyCode::Char('a') | KeyCode::Esc => Ok(Action::CloseSidePanel),
            KeyCode::Up => {
                self.cursor().prev();
                Ok(Action::None)
            }
            KeyCode::Down => {
                self.cursor().next();
                Ok(Action::None)
            }
            _ => Ok(Action::None),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let title = format!("Attribute - {}", self.file_name);
        let block = build_focused_block(&title);
        let label_style = Style::default().fg(Color::Yellow);
        let rows: Vec<Row> = self
            .entries
            .iter()
            .map(|(label, value)| {
                Row::new([
                    Cell::from(*label).style(label_style),
                    Cell::from(value.as_str()),
                ])
            })
            .collect();
        let table = Table::new(rows, [Constraint::Max(14), Constraint::Fill(1)])
            .block(block)
            .highlight_symbol("> ")
            .row_highlight_style(Style::default().add_modifier(Modifier::UNDERLINED));
        frame.render_stateful_widget(table, area, &mut self.table_state);
    }
}
