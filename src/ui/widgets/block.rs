use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Padding};

#[derive(Debug)]
pub enum BorderStyle {
    Active,
    Inactive,
    Error,
}

pub fn build_bordered_block(title: &str, style: BorderStyle) -> Block<'static> {
    let fg_color = match style {
        BorderStyle::Active => Color::Reset,
        BorderStyle::Inactive => Color::DarkGray,
        BorderStyle::Error => Color::Red,
    };
    Block::bordered()
        .title(title.to_string())
        .border_style(Style::default().fg(fg_color))
        .padding(Padding::horizontal(1))
}
