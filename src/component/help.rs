use crate::component::{Action, Component};
use crate::state::TextOutputState;
use crate::ui::widgets::render_text_output;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;

// キーバインド一覧をカテゴリ別に保持する。各要素は (カテゴリ名, そのカテゴリの項目).
// FilerComponent::handle_event のキーバインドと同期すること。
const KEY_BINDINGS: &[(&str, &[(&str, &str)])] = &[
    (
        "Navigation",
        &[
            ("Backspace", "Go to parent directory"),
            ("<", "Go back in directory history"),
            (">", "Go forward in directory history"),
            ("~", "Go to home directory"),
            ("j", "Jump to directory"),
            ("g", "Grep in files"),
        ],
    ),
    (
        "Selection & display",
        &[
            ("Space", "Toggle check mark"),
            (".", "Toggle dotfiles visibility"),
            ("s", "Sort files"),
            ("f", "Search files"),
        ],
    ),
    (
        "File operations",
        &[
            ("c", "Copy files"),
            ("m", "Move files"),
            ("d", "Delete files"),
            ("r", "Rename file"),
            ("k", "Create directory"),
            ("n", "Create file"),
            ("p", "Zip files"),
            ("u", "Unzip file"),
            ("x", "Execute command"),
            ("y", "Yank paths to clipboard"),
        ],
    ),
    (
        "Panels & views",
        &[
            ("a", "Show file attributes"),
            ("i", "Show file info"),
            ("t", "Show directory tree"),
            ("v", "Preview file"),
            ("h", "Launch shell"),
        ],
    ),
    (
        "Bookmarks",
        &[
            ("b", "Show bookmarks"),
            ("+", "Add bookmark"),
            ("-", "Remove bookmark"),
        ],
    ),
    (
        "App",
        &[("o", "Settings"), ("?", "Show this help"), ("q", "Quit")],
    ),
];

pub struct HelpComponent {
    text_output: TextOutputState,
}

impl HelpComponent {
    pub fn new() -> Self {
        // キー列の幅は全カテゴリ共通で揃える（説明の開始位置をパネル全体で統一する）。
        let key_width = KEY_BINDINGS
            .iter()
            .flat_map(|(_, entries)| entries.iter())
            .map(|(key, _)| key.len())
            .max()
            .unwrap_or(0);

        let mut lines = Vec::new();
        for (index, (category, entries)) in KEY_BINDINGS.iter().enumerate() {
            // カテゴリ間は空行で区切る。
            if index > 0 {
                lines.push(String::new());
            }
            // 見出しは左詰め、項目は字下げして「キー  説明」を整列。
            lines.push((*category).to_string());
            for (key, desc) in *entries {
                lines.push(format!("  {key:<key_width$}  {desc}"));
            }
        }

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
