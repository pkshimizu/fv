use crate::state::AttributeState;
use crate::ui::widgets::{BorderStyle, build_bordered_block};
use ratatui::layout::Constraint;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Cell, Row, Table};

pub fn build_attribute_table(state: &AttributeState) -> Table<'static> {
    let title = format!("Attribute - {}", state.file_name);
    let block = build_bordered_block(&title, BorderStyle::Active);
    let label_style = Style::default().fg(Color::Yellow);
    let rows: Vec<Row> = state
        .entries
        .iter()
        .map(|(label, value)| {
            Row::new([
                Cell::from(*label).style(label_style),
                Cell::from(value.clone()),
            ])
        })
        .collect();
    Table::new(rows, [Constraint::Max(14), Constraint::Fill(1)])
        .block(block)
        .highlight_symbol("> ")
        .row_highlight_style(Style::default().add_modifier(Modifier::UNDERLINED))
}
