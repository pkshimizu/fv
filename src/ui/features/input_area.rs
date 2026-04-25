use crate::state::InputMode;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Padding, Paragraph};

pub fn build_input_area(input: &InputMode) -> Paragraph {
    match input {
        InputMode::None => Paragraph::new(""),
        InputMode::Text { title, value, .. } | InputMode::File { title, value, .. } => {
            Paragraph::new(format!("{value}_")).block(
                Block::bordered()
                    .title(title.as_str())
                    .padding(Padding::horizontal(1)),
            )
        }
        InputMode::Select {
            title,
            options,
            selected_index,
            ..
        } => {
            let mut spans: Vec<Span> = Vec::new();
            for (i, opt) in options.iter().enumerate() {
                if i > 0 {
                    spans.push(Span::raw(" "));
                }
                if i == *selected_index {
                    spans.push(Span::styled(
                        format!("[{opt}]"),
                        Style::default().add_modifier(Modifier::REVERSED),
                    ));
                } else {
                    spans.push(Span::raw(format!(" {opt} ")));
                }
            }
            Paragraph::new(Line::from(spans)).block(
                Block::bordered()
                    .title(title.as_str())
                    .padding(Padding::horizontal(1)),
            )
        }
        InputMode::Confirm { title, .. } => Paragraph::new("Yes(y) No(n)").block(
            Block::bordered()
                .title(title.as_str())
                .padding(Padding::horizontal(1)),
        ),
        InputMode::Error { message } => Paragraph::new(message.as_str())
            .style(Style::default().fg(Color::Red))
            .block(
                Block::bordered()
                    .title("Error")
                    .border_style(Style::default().fg(Color::Red))
                    .padding(Padding::horizontal(1)),
            ),
    }
}
