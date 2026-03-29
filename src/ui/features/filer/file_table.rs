use crate::fs::{VFile, VFileTime};
use anyhow::Result;
use num_format::{Locale, ToFormattedString};
use ratatui::layout::{Alignment, Constraint};
use ratatui::style::{Modifier, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Cell, Row, Table};

fn format_time(time: Result<VFileTime>) -> String {
    if let Ok(time) = time {
        return time.to_string();
    }
    "____-__-__ --:--:--".to_string()
}

pub fn build_file_table(block: Block<'static>, files: &Vec<VFile>) -> Table<'static> {
    let rows: Vec<Row> = files
        .into_iter()
        .filter_map(|file| {
            let metadata = file.metadata().ok()?;
            Some(Row::new(vec![
                Cell::from(file.file_name().unwrap_or_default()),
                Cell::from(metadata.permissions().to_rwx_string()),
                Cell::from(
                    Text::from(if metadata.is_dir() {
                        "<dir>".to_string()
                    } else {
                        metadata.file_size().to_formatted_string(&Locale::en)
                    })
                    .alignment(Alignment::Right),
                ),
                Cell::from(format_time(metadata.modified())),
            ]))
        })
        .collect();
    Table::new(
        rows,
        [
            Constraint::Fill(1),
            Constraint::Max(6),
            Constraint::Max(10),
            Constraint::Max(19),
        ],
    )
    .block(block)
    .highlight_symbol("> ")
    .row_highlight_style(Style::default().add_modifier(Modifier::UNDERLINED))
}
