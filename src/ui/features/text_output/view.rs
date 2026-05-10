use crate::state::TextOutputState;
use crate::ui::widgets::{BorderStyle, build_bordered_block};
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Wrap};

pub fn build_text_output<'a>(state: &'a TextOutputState, title: &str) -> Paragraph<'a> {
    let title = if state.is_running() {
        format!("{title} ({}) Running", state.lines.len())
    } else {
        format!("{title} ({})", state.lines.len())
    };

    let (start, end, offset) = state.visible_range();
    let lines: Vec<Line<'a>> = state.lines[start..end]
        .iter()
        .map(|s| Line::from(s.as_str()))
        .collect();

    Paragraph::new(lines)
        .block(build_bordered_block(&title, BorderStyle::Active))
        .wrap(Wrap { trim: false })
        .scroll((offset, 0))
}
