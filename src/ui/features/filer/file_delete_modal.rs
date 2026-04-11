use crate::fs::VFile;
use crate::ui::features::modal::centered_rect;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::widgets::{Block, Clear, Paragraph};
use std::cmp::max;

pub fn render_delete_confirm_modal(frame: &mut Frame, area: Rect, file: &VFile) {
    let file_name = file.file_name().unwrap_or_default();

    let modal_area = centered_rect(max(50, (file_name.len() + 16) as u16), 6, area);
    frame.render_widget(Clear, modal_area);

    let block = Block::bordered().title("Confirm Delete");
    let inner_area = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let [message_area, _, action_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(inner_area);

    let message = Paragraph::new(format!("Delete \"{file_name}\"?")).alignment(Alignment::Center);
    frame.render_widget(message, message_area);

    let actions = Paragraph::new("[y] Yes  [n] No").alignment(Alignment::Center);
    frame.render_widget(actions, action_area);
}
