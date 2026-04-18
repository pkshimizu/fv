use crate::state::InputMode;
use ratatui::widgets::{Block, Paragraph};

pub fn build_input_area(input: &InputMode) -> Paragraph {
    match input {
        InputMode::None => Paragraph::new(""),
        InputMode::Text { title, value } => {
            Paragraph::new(format!("{value}_")).block(Block::bordered().title(title.as_str()))
        }
        InputMode::Confirm { title } | InputMode::DeleteConfirm { title, .. } => {
            Paragraph::new(" Yes(y) No(n))").block(Block::bordered().title(title.as_str()))
        }
    }
}
