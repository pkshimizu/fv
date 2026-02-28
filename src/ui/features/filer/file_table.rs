use ratatui::layout::{Alignment, Constraint};
use ratatui::text::Text;
use ratatui::widgets::{Block, Cell, Row, Table};
use std::fs::DirEntry;

pub fn build_file_table(
    block: Block<'static>,
    files: &Vec<std::io::Result<DirEntry>>,
) -> Table<'static> {
    let rows: Vec<Row> = files
        .into_iter()
        .map(|file| {
            Row::new(vec![
                Cell::from(
                    file.as_ref()
                        .unwrap()
                        .file_name()
                        .to_str()
                        .unwrap()
                        .to_string(),
                ),
                Cell::from(
                    Text::from(if file.as_ref().unwrap().metadata().unwrap().is_dir() {
                        "<dir>".to_string()
                    } else {
                        file.as_ref().unwrap().metadata().unwrap().len().to_string()
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
