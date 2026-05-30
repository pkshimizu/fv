use crate::component::{Action, Component};
use crate::fs::VFile;
use crate::fs::file_info_task::{FileInfoHandle, spawn_file_info};
use crate::state::TextOutputState;
use crate::ui::widgets::{Spinner, render_text_output};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use std::sync::mpsc::TryRecvError;

pub struct FileInfoComponent {
    /// 完了後に表示するタイトル（"File Info (N)" など）。loading 中はスピナー付きで上書き表示。
    title: String,
    text_output: TextOutputState,
    /// 取得中のみ `Some`。完了/エラー/受信者 drop で `None`（is_loading の判定にも使う）。
    handle: Option<FileInfoHandle>,
    spinner: Spinner,
}

impl FileInfoComponent {
    pub fn new(file: &VFile) -> Self {
        Self {
            title: String::new(),
            text_output: TextOutputState::with_lines(Vec::new()),
            handle: Some(spawn_file_info(file)),
            spinner: Spinner::new(),
        }
    }

    pub fn is_loading(&self) -> bool {
        self.handle.is_some()
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

    fn tick(&mut self) {
        self.spinner.advance();
        let Some(handle) = self.handle.as_ref() else {
            return;
        };
        match handle.rx.try_recv() {
            Ok(Ok(info)) => {
                let lines = info.to_lines();
                self.title = format!("File Info ({})", lines.len());
                self.text_output = TextOutputState::with_lines(lines);
                self.handle = None;
            }
            Ok(Err(e)) => {
                self.title = "File Info (error)".to_string();
                self.text_output = TextOutputState::with_lines(vec![format!("Error: {e}")]);
                self.handle = None;
            }
            Err(TryRecvError::Empty) => {} // 取得中
            Err(TryRecvError::Disconnected) => {
                self.title = "File Info (error)".to_string();
                self.text_output =
                    TextOutputState::with_lines(vec!["Error: file info task ended".to_string()]);
                self.handle = None;
            }
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        if self.is_loading() {
            let title = format!("File Info {} Loading", self.spinner.frame());
            render_text_output(frame, area, &mut self.text_output, &title);
            return;
        }
        render_text_output(frame, area, &mut self.text_output, &self.title);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::TempDir;

    fn render_to_string(comp: &mut FileInfoComponent, width: u16, height: u16) -> String {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("terminal");
        terminal
            .draw(|frame| {
                let a = frame.area();
                comp.render(frame, a);
            })
            .expect("draw");
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn file_info_component_transitions_from_loading_to_loaded() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("note.txt");
        std::fs::write(&path, b"hello\n").unwrap();
        let file = VFile::new(path.to_str().unwrap());

        let mut comp = FileInfoComponent::new(&file);
        assert!(comp.is_loading(), "should start in loading state");

        let mut ticks = 0;
        while comp.is_loading() && ticks < 5_000 {
            comp.tick();
            std::thread::sleep(Duration::from_millis(1));
            ticks += 1;
        }
        assert!(!comp.is_loading(), "should finish loading");

        let text = render_to_string(&mut comp, 80, 12);
        assert!(
            text.contains("Size"),
            "loaded content expected, got: {text:?}"
        );
        assert!(
            !text.contains("Loading"),
            "loading label should be gone, got: {text:?}"
        );
    }
}
