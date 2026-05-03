use crate::state::PromptMode;
use crate::ui::widgets::{BorderStyle, build_bordered_block};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

pub fn build_prompt_view(mode: &PromptMode) -> Paragraph {
    match mode {
        PromptMode::None => {
            Paragraph::new("q: Quit").block(build_bordered_block("Commands", BorderStyle::Inactive))
        }
        PromptMode::Text { title, value, .. }
        | PromptMode::File { title, value, .. }
        | PromptMode::Search { title, value, .. } => Paragraph::new(format!("{value}_"))
            .block(build_bordered_block(title.as_ref(), BorderStyle::Active)),
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
            Paragraph::new(Line::from(spans))
                .block(build_bordered_block(title.as_str(), BorderStyle::Active))
        }
        PromptMode::Confirm { title, .. } => Paragraph::new("Yes(y) No(n)")
            .block(build_bordered_block(title.as_str(), BorderStyle::Active)),
        PromptMode::Error { message } => Paragraph::new(message.as_str())
            .style(Style::default().fg(Color::Red))
            .block(build_bordered_block("Error", BorderStyle::Error)),
    }
}
