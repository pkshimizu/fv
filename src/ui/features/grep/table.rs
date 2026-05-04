use crate::state::GrepState;
use crate::ui::widgets::{BorderStyle, build_bordered_block};
use ratatui::layout::Constraint;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Cell, Row, Table};

pub fn build_grep_table(state: &GrepState) -> Table<'static> {
    let paths = &state.paths;
    let block = build_bordered_block(&format!("Grep ({})", paths.len()), BorderStyle::Active);
    let rows: Vec<Row> = paths
        .iter()
        .map(|path| Row::new([Cell::from(path.clone())]))
        .collect();
    Table::new(rows, [Constraint::Fill(1)])
        .block(block)
        .highlight_symbol("> ")
        .row_highlight_style(Style::default().add_modifier(Modifier::UNDERLINED))
}
