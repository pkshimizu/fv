use crate::state::PromptMode;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Padding, Paragraph};

pub fn build_prompt_view(input: &PromptMode) -> Paragraph {
    match input {
        PromptMode::None => {
            Paragraph::new("q: Quit").block(Block::bordered().padding(Padding::horizontal(1)))
        }
        PromptMode::Text { title, value, .. }
        | PromptMode::File { title, value, .. }
        | PromptMode::Search { title, value, .. } => Paragraph::new(format!("{value}_")).block(
            Block::bordered()
                .title(title.as_str())
                .padding(Padding::horizontal(1)),
        ),
        PromptMode::Select {
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
        PromptMode::Confirm { title, .. } => Paragraph::new("Yes(y) No(n)").block(
            Block::bordered()
                .title(title.as_str())
                .padding(Padding::horizontal(1)),
        ),
        PromptMode::Error { message } => Paragraph::new(message.as_str())
            .style(Style::default().fg(Color::Red))
            .block(
                Block::bordered()
                    .title("Error")
                    .border_style(Style::default().fg(Color::Red))
                    .padding(Padding::horizontal(1)),
            ),
    }
}
