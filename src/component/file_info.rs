use crate::component::{Action, Component};
use crate::fs::VFile;
use crate::fs::file_info::FileInfo;
use crate::state::TextOutputState;
use crate::ui::widgets::render_text_output;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;

pub struct FileInfoComponent {
    title: String,
    text_output: TextOutputState,
}

impl FileInfoComponent {
    pub fn new(file: &VFile) -> Result<Self> {
        let info = FileInfo::from_file(file)?;
        let lines = info.to_lines();
        let title = format!("File Info ({})", lines.len());
        let text_output = TextOutputState::with_lines(lines);
        Ok(Self { title, text_output })
    }
}

impl Component for FileInfoComponent {
    fn keymap(&self) -> &'static str {
        "↑↓: Scroll  ←→: Top/Bottom  i/Esc: Close"
    }

    fn handle_event(&mut self, event: KeyEvent) -> Result<Action> {
        if self.text_output.handle_scroll_key(event.code) {
            return Ok(Action::None);
        }
        match event.code {
            KeyCode::Char('i') | KeyCode::Esc => Ok(Action::CloseSidePanel),
            _ => Ok(Action::None),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        render_text_output(frame, area, &mut self.text_output, &self.title);
    }
}
