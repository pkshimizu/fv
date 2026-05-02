use crate::fs::VFileTime;
use crate::state::FilerState;
use crate::store::RootStore;
use anyhow::Result;
use num_format::{Locale, ToFormattedString};
use ratatui::layout::{Alignment, Constraint};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Cell, Row, Table};

const DOTFILE_STYLE: Style = Style::new().fg(Color::Blue);
const DIR_STYLE: Style = Style::new().fg(Color::Green);

fn format_time(time: Result<VFileTime>) -> String {
    if let Ok(time) = time {
        return time.to_string();
    }
    "____-__-__ --:--:--".to_string()
}

pub fn build_file_table(
    block: Block<'static>,
    filer_state: &FilerState,
    store: &RootStore,
) -> Table<'static> {
    let files = &filer_state.current_dir_files;
    let rows: Vec<Row> = files
        .iter()
        .filter_map(|file| {
            let metadata = file.metadata().ok()?;
            let checked = if filer_state.is_checked(file) {
                "*"
            } else {
                " "
            };
            let file_name = file.file_name().unwrap_or_default();
            let is_dotfile = file_name.starts_with('.');
            let is_dir = metadata.is_dir();
            let is_bookmarked = store.bookmark.has(file.absolute_path());
            let row = Row::new(vec![
                Cell::from(checked),
                Cell::from(file_name.to_string()),
                Cell::from(Text::from(if is_bookmarked {
                    "B".to_string()
                } else {
                    " ".to_string()
                })),
                Cell::from(metadata.permissions().to_rwx_string()),
                Cell::from(
                    Text::from(if is_dir {
                        "<dir>".to_string()
                    } else {
                        metadata.file_size().to_formatted_string(&Locale::en)
                    })
                    .alignment(Alignment::Right),
                ),
                Cell::from(format_time(metadata.modified())),
            ]);
            let row = if is_dir {
                row.style(DIR_STYLE)
            } else if is_dotfile {
                row.style(DOTFILE_STYLE)
            } else {
                row
            };
            Some(row)
        })
        .collect();
    Table::new(
        rows,
        [
            Constraint::Max(1),
            Constraint::Fill(1),
            Constraint::Max(1),
            Constraint::Max(6),
            Constraint::Max(10),
            Constraint::Max(19),
        ],
    )
    .block(block)
    .highlight_symbol("> ")
    .row_highlight_style(Style::default().add_modifier(Modifier::UNDERLINED))
}
