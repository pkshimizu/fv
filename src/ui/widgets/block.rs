use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Padding};

pub fn build_bordered_block(title: &str, is_active: bool) -> Block<'static> {
    let fg_color = if is_active {
        Color::White
    } else {
        Color::DarkGray
    };
    Block::bordered()
        .title(title.to_string())
        .border_style(Style::default().fg(fg_color))
        .padding(Padding::horizontal(1))
}
