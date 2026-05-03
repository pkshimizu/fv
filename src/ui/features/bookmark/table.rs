use crate::state::BookmarkState;
use crate::ui::widgets::build_bordered_block;
use ratatui::layout::Constraint;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Cell, Row, Table};

pub fn build_bookmark_table(state: &BookmarkState) -> Table<'static> {
    let paths = &state.paths;
    let block = build_bordered_block(format!("Bookmarks ({})", paths.len()).as_str(), true);
    let rows: Vec<Row> = paths
        .iter()
        .map(|path| Row::new([Cell::from(path.clone())]))
        .collect();
    Table::new(rows, [Constraint::Fill(1)])
        .block(block)
        .highlight_symbol("> ")
        .row_highlight_style(Style::default().add_modifier(Modifier::UNDERLINED))
}
