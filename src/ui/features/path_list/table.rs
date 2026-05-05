use crate::state::PathListState;
use crate::ui::widgets::{BorderStyle, build_bordered_block};
use ratatui::layout::Constraint;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Cell, Row, Table};

pub fn build_path_table(state: &PathListState, title: &str) -> Table<'static> {
    let paths = &state.paths;
    let is_running = state.is_running();
    let block_title = if is_running {
        format!("{} ({}) Running", title, paths.len())
    } else {
        format!("{} ({})", title, paths.len())
    };
    let block = build_bordered_block(&block_title, BorderStyle::Active);
    let rows = paths
        .iter()
        .map(|path| Row::new([Cell::from(path.clone())]));
    Table::new(rows, [Constraint::Fill(1)])
        .block(block)
        .highlight_symbol("> ")
        .row_highlight_style(Style::default().add_modifier(Modifier::UNDERLINED))
}
