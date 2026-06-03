use crate::component::{Action, Component, handle_preview_common_key};
use crate::fs::text_preview::TextPreview;
use crate::state::TextOutputState;
use crate::ui::widgets::render_text_output;
use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

/// プレビューパネルのタイトル文字列を組み立てる。
fn preview_title(file_name: &str) -> String {
    format!("Preview - {file_name}")
}

pub struct PreviewComponent {
    title: String,
    text_output: TextOutputState,
}

impl PreviewComponent {
    pub fn new(path: &str, file_name: &str) -> Result<Self> {
        let preview = TextPreview::from_file(path)?;
        let title = if preview.truncated {
            format!("{} (truncated)", preview_title(file_name))
        } else {
            preview_title(file_name)
        };
        let text_output = TextOutputState::with_lines(preview.lines);
        Ok(Self { title, text_output })
    }

    /// マークダウンファイル（`.md` / `.markdown`）をレンダリングしてプレビューする。
    /// 読み込み上限・バイナリ判定はテキストプレビューと同じ `TextPreview` を流用する。
    pub fn new_markdown(path: &str, file_name: &str) -> Result<Self> {
        let preview = TextPreview::from_file(path)?;
        let title = if preview.truncated {
            format!("{} (truncated)", preview_title(file_name))
        } else {
            preview_title(file_name)
        };
        let source = preview.lines.join("\n");
        let lines = crate::ui::markdown::render(&source);
        let text_output = TextOutputState::with_styled_lines(lines);
        Ok(Self { title, text_output })
    }

    /// プレビューできないファイル（ディレクトリ・バイナリ・読み込み失敗等）に対して、
    /// 理由メッセージをサイドパネル内に表示するためのコンポーネントを作る。
    pub fn with_message(file_name: &str, text: impl Into<String>) -> Self {
        Self {
            title: preview_title(file_name),
            text_output: TextOutputState::with_lines(vec![text.into()]),
        }
    }
}

impl Component for PreviewComponent {
    fn keymap(&self) -> &'static str {
        "↑↓: Scroll  ←→: Top/Bottom  n/p: Next/Prev  v/Esc: Close"
    }

    fn handle_event(&mut self, event: KeyEvent) -> Result<Action> {
        if self.text_output.handle_scroll_key(event.code) {
            return Ok(Action::None);
        }
        if let Some(action) = handle_preview_common_key(event.code) {
            return Ok(action);
        }
        Ok(Action::None)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        render_text_output(frame, area, &mut self.text_output, &self.title);
    }
}
