use crate::component::{Action, Component};
use crate::ui::widgets::{BorderStyle, build_bordered_block};
use anyhow::{Context, Result};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui_image::StatefulImage;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;

pub struct ImagePreviewComponent {
    title: String,
    protocol: StatefulProtocol,
}

impl ImagePreviewComponent {
    pub fn new(path: &str, file_name: &str, picker: &Picker) -> Result<Self> {
        let dyn_img = image::open(path).with_context(|| format!("Failed to open image {path}"))?;
        let protocol = picker.new_resize_protocol(dyn_img);
        let title = format!("Preview - {file_name}");
        Ok(Self { title, protocol })
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

        let image = StatefulImage::default();
        frame.render_stateful_widget(image, inner, &mut self.protocol);
    }
}
