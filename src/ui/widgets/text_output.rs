use crate::state::TextOutputState;
use crate::ui::widgets::build_focused_block;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

pub fn render_text_output(frame: &mut Frame, area: Rect, state: &mut TextOutputState, title: &str) {
    state.set_visible_area(area.height.saturating_sub(2), area.width.saturating_sub(2));

    let (start, end, offset) = state.visible_range();
    // 可視範囲のみを描画用に組み直す。各 Span の文字列は所有値を複製せず借用するため、
    // 毎フレームのディープコピーを避けられる（保持元 `state.lines` の寿命内で借用）。
    let lines: Vec<Line> = state.lines[start..end]
        .iter()
        .map(|line| {
            let spans: Vec<Span> = line
                .spans
                .iter()
                .map(|s| Span::styled(s.content.as_ref(), s.style))
                .collect();
            let mut rebuilt = Line::from(spans);
            rebuilt.style = line.style;
            rebuilt.alignment = line.alignment;
            rebuilt
        })
        .collect();

    let paragraph = Paragraph::new(lines)
        .block(build_focused_block(title))
        .wrap(Wrap { trim: false })
        .scroll((offset, 0));
    frame.render_widget(paragraph, area);
}
