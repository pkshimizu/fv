use crate::state::BookmarkListState;
use ratatui::layout::Constraint;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Cell, Row, Table};

pub fn build_bookmark_list(state: &BookmarkListState) -> Table<'static> {
    let block = Block::bordered().title(format!("Bookmarks ({})", state.paths.len()));
    let rows: Vec<Row> = state
        .paths
        .iter()
        .map(|path| Row::new([Cell::from(path.clone())]))
        .collect();
    Table::new(rows, [Constraint::Fill(1)])
        .block(block)
        .highlight_symbol("> ")
        .row_highlight_style(Style::default().add_modifier(Modifier::UNDERLINED))
}
