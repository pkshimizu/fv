use crate::fs::VFile;
use ratatui::layout::{Alignment, Constraint};
use ratatui::text::Text;
use ratatui::widgets::{Block, Cell, Row, Table};

pub fn build_file_table(block: Block<'static>, files: &Vec<VFile>) -> Table<'static> {
    let rows: Vec<Row> = files
        .into_iter()
        .map(|file| {
            Row::new(vec![
                Cell::from(file.file_name()),
                Cell::from(
                    Text::from(if file.is_dir() {
                        "<dir>".to_string()
                    } else {
                        file.file_size().to_string()
                    })
                    .alignment(Alignment::Right),
                ),
            ])
        })
        .collect();
    Table::new(rows, [Constraint::Fill(1), Constraint::Max(10)])
        .block(block)
        .highlight_symbol("> ")
}
