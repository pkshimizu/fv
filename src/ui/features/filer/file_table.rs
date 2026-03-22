use crate::fs::VFile;
use chrono::{DateTime, Datelike, Local, Timelike};
use num_format::{Locale, ToFormattedString};
use ratatui::layout::{Alignment, Constraint};
use ratatui::style::{Modifier, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Cell, Row, Table};
use std::io;
use std::time::SystemTime;

fn format_system_time(time: io::Result<SystemTime>) -> String {
    if let Ok(time) = time {
        let utc_time: DateTime<Local> = time.into();
        return format!(
            "{}-{:02}-{:02} {:02}:{:02}:{:02}",
            utc_time.year(),
            utc_time.month(),
            utc_time.day(),
            utc_time.hour(),
            utc_time.minute(),
            utc_time.second()
        );
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
                Cell::from(format_system_time(file.modified())),
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
