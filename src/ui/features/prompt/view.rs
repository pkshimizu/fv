use crate::state::PromptMode;
use crate::ui::widgets::build_bordered_block;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Padding, Paragraph};

pub fn build_prompt_view(mode: &PromptMode) -> Paragraph {
    match mode {
        PromptMode::None => {
            Paragraph::new("q: Quit").block(build_bordered_block("Commands", false))
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
            Paragraph::new(Line::from(spans)).block(build_bordered_block(title.as_str(), true))
        }
        PromptMode::Confirm { title, .. } => {
            Paragraph::new("Yes(y) No(n)").block(build_bordered_block(title.as_str(), true))
        }
        PromptMode::Error { message } => Paragraph::new(message.as_str())
            .style(Style::default().fg(Color::Red))
            .block(build_bordered_block("Error", true)),
    }
}
