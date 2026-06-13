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
            ("shift + a", "Select all / clear selection"),
            (".", "Toggle dotfiles visibility"),
            ("s", "Sort files"),
            ("f", "Search files"),
            ("/", "Filter list (hide non-matches)"),
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
            ("l", "Create symlink to cursor file"),
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
            ("e", "Open in file manager"),
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
        Self {
            text_output: TextOutputState::with_lines(Self::build_lines()),
        }
    }

    /// KEY_BINDINGS から、カテゴリ見出し＋整列したキー一覧の表示行を組み立てる。
    fn build_lines() -> Vec<String> {
        // キー列の幅は全カテゴリ共通で揃える（説明の開始位置をパネル全体で統一する）。
        // キーはすべて ASCII 前提のため、バイト長 = 表示桁数として `len()` を用いる。
        let key_width = KEY_BINDINGS
            .iter()
            .flat_map(|(_, entries)| entries.iter())
            .map(|(key, _)| key.len())
            .max()
            .unwrap_or(0);

        // 行数（見出し + 各項目 + カテゴリ間の空行）を事前に見積もって確保する。
        let entry_count: usize = KEY_BINDINGS.iter().map(|(_, entries)| entries.len()).sum();
        let capacity = KEY_BINDINGS.len() + entry_count + KEY_BINDINGS.len().saturating_sub(1);
        let mut lines = Vec::with_capacity(capacity);

        for (index, (category, entries)) in KEY_BINDINGS.iter().enumerate() {
            // カテゴリ間は空行で区切る。
            if index > 0 {
                lines.push(String::new());
            }
            // 見出しは左詰め、項目は字下げして「キー  説明」を整列。
            lines.push(category.to_string());
            for (key, desc) in *entries {
                lines.push(format!("  {key:<key_width$}  {desc}"));
            }
        }

        lines
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 同じキーが複数のカテゴリに重複して登録されていないこと。
    #[test]
    fn key_bindings_have_no_duplicate_keys() {
        let mut keys: Vec<&str> = KEY_BINDINGS
            .iter()
            .flat_map(|(_, entries)| entries.iter().map(|(key, _)| *key))
            .collect();
        let total = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), total, "重複したキーが存在します");
    }

    /// すべてのカテゴリ見出しが、独立した行として出力されること。
    #[test]
    fn build_lines_includes_every_category_heading() {
        let lines = HelpComponent::build_lines();
        for (category, _) in KEY_BINDINGS {
            assert!(
                lines.iter().any(|line| line == category),
                "カテゴリ見出しが見つかりません: {category}"
            );
        }
    }

    /// すべてのキー・説明が、いずれかの行に整列して含まれること。
    #[test]
    fn build_lines_includes_every_binding() {
        let lines = HelpComponent::build_lines();
        for (_, entries) in KEY_BINDINGS {
            for (key, desc) in *entries {
                assert!(
                    lines
                        .iter()
                        .any(|line| line.contains(key) && line.contains(desc)),
                    "キーバインドが見つかりません: {key} {desc}"
                );
            }
        }
    }

    /// カテゴリ間が空行で区切られていること（空行数 = カテゴリ数 - 1）。
    #[test]
    fn categories_are_separated_by_blank_lines() {
        let lines = HelpComponent::build_lines();
        let blank_count = lines.iter().filter(|line| line.is_empty()).count();
        assert_eq!(blank_count, KEY_BINDINGS.len().saturating_sub(1));
    }
}
