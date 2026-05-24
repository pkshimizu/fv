use crate::component::{Action, Component};
use crate::ui::widgets::{BorderStyle, build_bordered_block};
use anyhow::{Context, Result};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui_image::StatefulImage;
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::protocol::StatefulProtocol;

pub struct ImagePreviewComponent {
    title: String,
    protocol: StatefulProtocol,
    is_halfblocks: bool,
}

impl ImagePreviewComponent {
    pub fn new(path: &str, file_name: &str, picker: &Picker) -> Result<Self> {
        let dyn_img = image::open(path).with_context(|| format!("Failed to open image {path}"))?;
        let is_halfblocks = picker.protocol_type() == ProtocolType::Halfblocks;
        let protocol = picker.new_resize_protocol(dyn_img);
        let title = format!("Preview - {file_name}");
        Ok(Self {
            title,
            protocol,
            is_halfblocks,
        })
    }
}

impl Component for ImagePreviewComponent {
    fn handle_event(&mut self, event: KeyEvent) -> Result<Action> {
        match event.code {
            KeyCode::Char('v') | KeyCode::Esc => Ok(Action::CloseSidePanel),
            _ => Ok(Action::None),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let block = build_bordered_block(&self.title, BorderStyle::Active);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.is_halfblocks {
            let chunks = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(inner);
            let notice = Paragraph::new(Line::from(
                "Terminal does not support image protocol. Display quality is limited.",
            ))
            .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(notice, chunks[0]);
            let image = StatefulImage::default();
            frame.render_stateful_widget(image, chunks[1], &mut self.protocol);
        } else {
            let image = StatefulImage::default();
            frame.render_stateful_widget(image, inner, &mut self.protocol);
        }
    }
}
