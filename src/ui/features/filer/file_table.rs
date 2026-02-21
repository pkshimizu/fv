use ratatui::layout::Constraint;
use ratatui::widgets::{Block, Cell, Row, Table};
use std::fs::DirEntry;

pub fn build_file_table(
    block: Block<'static>,
    files: &Vec<std::io::Result<DirEntry>>,
) -> Table<'static> {
    let rows: Vec<Row> = files
        .into_iter()
        .map(|file| {
            Row::new(vec![Cell::from(
                file.as_ref()
                    .unwrap()
                    .file_name()
                    .to_str()
                    .unwrap()
                    .to_string(),
            )])
        })
        .collect();
    Table::new(rows, [Constraint::Fill(1)])
        .block(block)
        .highlight_symbol("> ")
}
