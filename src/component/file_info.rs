use crate::component::{Action, Component};
use crate::fs::VFile;
use crate::fs::file_info::FileInfo;
use crate::state::TextOutputState;
use crate::ui::widgets::{BorderStyle, build_bordered_block};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Wrap};

pub struct FileInfoComponent {
    state: TextOutputState,
}

impl FileInfoComponent {
    pub fn new(file: &VFile) -> Result<Self> {
        let info = FileInfo::from_file(file)?;
        let mut state = TextOutputState::new();
        state.lines = info.to_lines();
        Ok(Self { state })
    }
}

impl Component for FileInfoComponent {
    fn handle_event(&mut self, event: KeyEvent) -> Result<Action> {
        match event.code {
            KeyCode::Char('i') | KeyCode::Esc => Ok(Action::CloseSidePanel),
            KeyCode::Up => {
                self.state.scroll_up();
                Ok(Action::None)
            }
            KeyCode::Down => {
                self.state.scroll_down();
                Ok(Action::None)
            }
            KeyCode::Left => {
                self.state.scroll_to_top();
                Ok(Action::None)
            }
            KeyCode::Right => {
                self.state.scroll_to_bottom();
                Ok(Action::None)
            }
            _ => Ok(Action::None),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.state
            .set_visible_area(area.height.saturating_sub(2), area.width.saturating_sub(2));

        let title = format!("File Info ({})", self.state.lines.len());
        let (start, end, offset) = self.state.visible_range();
        let lines: Vec<Line> = self.state.lines[start..end]
            .iter()
            .map(|s| Line::from(s.as_str()))
            .collect();

        let paragraph = Paragraph::new(lines)
            .block(build_bordered_block(&title, BorderStyle::Active))
            .wrap(Wrap { trim: false })
            .scroll((offset, 0));
        frame.render_widget(paragraph, area);
    }
}
