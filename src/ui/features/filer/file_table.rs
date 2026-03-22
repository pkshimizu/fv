use crate::fs::{VFile, VFileTime};
use num_format::{Locale, ToFormattedString};
use ratatui::layout::{Alignment, Constraint};
use ratatui::style::{Modifier, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Cell, Row, Table};

fn format_time(time: Result<VFileTime, ()>) -> String {
    if let Ok(time) = time {
        return time.to_string()
    }
    "____-__-__ --:--:--".to_string()
}

pub fn build_file_table(block: Block<'static>, files: &Vec<VFile>) -> Table<'static> {
    let rows: Vec<Row> = files
        .into_iter()
        .map(|file| {
            Row::new(vec![
                Cell::from(file.file_name()),
                Cell::from(if let Ok(permissions) = file.permissions() {
                    permissions.to_rwx_string()
                } else {
                    "------".to_string()
                }),
                Cell::from(
                    Text::from(if file.is_dir() {
                        "<dir>".to_string()
                    } else {
                        file.file_size().to_formatted_string(&Locale::en)
                    })
                    .alignment(Alignment::Right),
                ),
                Cell::from(format_time(file.modified())),
            ])
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
