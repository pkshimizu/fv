use crate::state::TextOutputState;
use crate::ui::widgets::{BorderStyle, build_bordered_block};
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Wrap};

pub fn build_text_output(state: &TextOutputState, title: &str) -> Paragraph<'static> {
    let title = if state.is_running() {
        format!("{title} ({}) Running", state.lines.len())
    } else {
        format!("{title} ({})", state.lines.len())
    };

    let lines: Vec<Line<'static>> = state.lines.iter().map(|s| Line::from(s.clone())).collect();

    Paragraph::new(lines)
        .block(build_bordered_block(&title, BorderStyle::Active))
        .wrap(Wrap { trim: false })
        .scroll((state.scroll_offset, 0))
}
