use crate::fs::VFile;
use crate::ui::features::modal::centered_rect;
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::widgets::{Block, Clear, Paragraph};
use std::cmp::max;

pub fn render_delete_confirm_modal(frame: &mut Frame, area: Rect, file: &VFile) {
    let file_name = file.file_name().unwrap_or_default();

    let modal_area = centered_rect(max(50, (file_name.len() + 16) as u16), 5, area);
    frame.render_widget(Clear, modal_area);

    let text = format!("Delete \"{file_name}\"?\n\n[y] Yes  [n] No");
    let widget = Paragraph::new(text)
        .block(Block::bordered().title("Confirm Delete"))
        .alignment(Alignment::Center);
    frame.render_widget(widget, modal_area);
}
