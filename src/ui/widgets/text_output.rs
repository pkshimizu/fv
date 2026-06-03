use crate::state::TextOutputState;
use crate::ui::widgets::build_focused_block;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{Paragraph, Wrap};

pub fn render_text_output(frame: &mut Frame, area: Rect, state: &mut TextOutputState, title: &str) {
    state.set_visible_area(area.height.saturating_sub(2), area.width.saturating_sub(2));

    let (start, end, offset) = state.visible_range();
    let lines = state.lines[start..end].to_vec();

    let paragraph = Paragraph::new(lines)
        .block(build_focused_block(title))
        .wrap(Wrap { trim: false })
        .scroll((offset, 0));
    frame.render_widget(paragraph, area);
}
