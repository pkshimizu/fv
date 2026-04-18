use crate::state::InputMode;
use ratatui::widgets::{Block, Padding, Paragraph};

pub fn build_input_area(input: &InputMode) -> Paragraph {
    match input {
        InputMode::None => Paragraph::new(""),
        InputMode::Text { title, value, .. } => Paragraph::new(format!("{value}_")).block(
            Block::bordered()
                .title(title.as_str())
                .padding(Padding::horizontal(1)),
        ),
        InputMode::Confirm { title, .. } => Paragraph::new("Yes(y) No(n)").block(
            Block::bordered()
                .title(title.as_str())
                .padding(Padding::horizontal(1)),
        ),
    }
}
