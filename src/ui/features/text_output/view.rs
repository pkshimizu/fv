use crate::state::TextOutputState;
use crate::ui::widgets::{BorderStyle, build_bordered_block};
use ratatui::text::Text;
use ratatui::widgets::{Paragraph, Wrap};

pub fn build_text_output(state: &TextOutputState, title: &str) -> Paragraph<'static> {
    let title = if state.is_running() {
        format!("{title} ({}) Running", state.lines.len())
    } else {
        format!("{title} ({})", state.lines.len())
    };

    let text = Text::from(state.lines.join("\n"));

    Paragraph::new(text)
        .block(build_bordered_block(&title, BorderStyle::Active))
        .wrap(Wrap { trim: false })
        .scroll((state.scroll_offset, 0))
}
