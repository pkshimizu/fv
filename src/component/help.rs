use crate::component::{Action, Component};
use crate::state::TextOutputState;
use crate::ui::widgets::render_text_output;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;

// キーバインド一覧。FilerComponent::handle_event のキーバインドと同期すること。
const KEY_BINDINGS: &[(&str, &str)] = &[
    ("Backspace", "Go to parent directory"),
    ("Space", "Toggle check mark"),
    (".", "Toggle dotfiles visibility"),
    ("a", "Show file attributes"),
    ("b", "Show bookmarks"),
    ("c", "Copy files"),
    ("d", "Delete files"),
    ("f", "Search files"),
    ("g", "Grep in files"),
    ("h", "Launch shell"),
    ("i", "Show file info"),
    ("j", "Jump to directory"),
    ("k", "Create directory"),
    ("m", "Move files"),
    ("n", "Create file"),
    ("o", "Settings"),
    ("p", "Zip files"),
    ("r", "Rename file"),
    ("s", "Sort files"),
    ("t", "Show directory tree"),
    ("u", "Unzip file"),
    ("v", "Preview file"),
    ("x", "Execute command"),
    ("<", "Go back in directory history"),
    (">", "Go forward in directory history"),
    ("+", "Add bookmark"),
    ("-", "Remove bookmark"),
    ("?", "Show this help"),
    ("q", "Quit"),
];

pub struct HelpComponent {
    text_output: TextOutputState,
}

impl HelpComponent {
    pub fn new() -> Self {
        let key_width = KEY_BINDINGS.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
        let lines = KEY_BINDINGS
            .iter()
            .map(|(key, desc)| format!("  {key:<key_width$}  {desc}"))
            .collect();
        Self {
            text_output: TextOutputState::with_lines(lines),
        }
    }
}

impl Component for HelpComponent {
    fn handle_event(&mut self, event: KeyEvent) -> Result<Action> {
        if self.text_output.handle_scroll_key(event.code) {
            return Ok(Action::None);
        }
        match event.code {
            KeyCode::Char('?') | KeyCode::Esc => Ok(Action::CloseSidePanel),
            _ => Ok(Action::None),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        render_text_output(frame, area, &mut self.text_output, "Help");
    }

    fn keymap(&self) -> &'static str {
        "↑↓: Scroll  ←→: Top/Bottom  ?/Esc: Close"
    }
}
