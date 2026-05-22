use crate::component::{Action, Component};
use crate::fs::text_preview::TextPreview;
use crate::state::TextOutputState;
use crate::ui::widgets::render_text_output;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;

pub struct PreviewComponent {
    title: String,
    text_output: TextOutputState,
}

impl PreviewComponent {
    pub fn new(path: &str, file_name: &str) -> Result<Self> {
        let preview = TextPreview::from_file(path)?;
        let title = if preview.truncated {
            format!("Preview - {file_name} (truncated)")
        } else {
            format!("Preview - {file_name}")
        };
        let text_output = TextOutputState::with_lines(preview.lines);
        Ok(Self { title, text_output })
    }
}

impl Component for PreviewComponent {
    fn handle_event(&mut self, event: KeyEvent) -> Result<Action> {
        if self.text_output.handle_scroll_key(event.code) {
            return Ok(Action::None);
        }
        match event.code {
            KeyCode::Char('v') | KeyCode::Esc => Ok(Action::CloseSidePanel),
            _ => Ok(Action::None),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        render_text_output(frame, area, &mut self.text_output, &self.title);
    }
}
