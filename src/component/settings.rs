use crate::component::{Action, Component};
use crate::store::StartupDirectory;
use crate::ui::widgets::build_focused_block;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

/// Specific Directory 選択肢のインデックス（`StartupDirectory::LABELS` の末尾）。
const SPECIFIC_INDEX: usize = 3;

/// 設定パネル内のフォーカス対象。
#[derive(PartialEq, Eq)]
enum Focus {
    /// Startup Directory のラジオ行。
    Options,
    /// Specific Directory のパス入力フィールド。
    Path,
}

pub struct SettingsComponent {
    /// 初期値のインデックス
    initial_option: usize,
    /// 選択中のオプションインデックス
    selected_option: usize,
    /// Specific Directory のパス入力バッファ。ラジオを他へ切り替えても保持し、
    /// 戻したときに入力中の内容が消えないようにする。
    path: String,
    /// 初期パス（dirty 判定用）。
    initial_path: String,
    focus: Focus,
}

impl SettingsComponent {
    pub fn new(startup_dir: &StartupDirectory) -> Self {
        let index = startup_dir.index();
        let path = match startup_dir {
            StartupDirectory::SpecificDirectory(p) => p.clone(),
            _ => String::new(),
        };
        Self {
            initial_option: index,
            selected_option: index,
            initial_path: path.clone(),
            path,
            focus: Focus::Options,
        }
    }

    fn is_specific_selected(&self) -> bool {
        self.selected_option == SPECIFIC_INDEX
    }

    fn is_dirty(&self) -> bool {
        if self.selected_option != self.initial_option {
            return true;
        }
        // 選択肢が同じでも、Specific のパスを編集していれば dirty。
        self.is_specific_selected() && self.path != self.initial_path
    }

    fn to_startup_directory(&self) -> StartupDirectory {
        match self.selected_option {
            1 => StartupDirectory::HomeDirectory,
            2 => StartupDirectory::LastDirectory,
            SPECIFIC_INDEX => StartupDirectory::SpecificDirectory(self.path.clone()),
            _ => StartupDirectory::CurrentDirectory,
        }
    }

    fn save_or_close(&self) -> Action {
        if self.is_dirty() {
            Action::SaveSettings(Box::new(self.to_startup_directory()))
        } else {
            Action::CloseSidePanel
        }
    }

    fn handle_options_key(&mut self, code: KeyCode) -> Action {
        match code {
            KeyCode::Char('o') | KeyCode::Esc => return self.save_or_close(),
            KeyCode::Left => {
                self.selected_option = self.selected_option.saturating_sub(1);
            }
            KeyCode::Right if self.selected_option + 1 < StartupDirectory::LABELS.len() => {
                self.selected_option += 1;
            }
            // Specific 選択時のみパス入力へフォーカスを移す。
            KeyCode::Down if self.is_specific_selected() => {
                self.focus = Focus::Path;
            }
            _ => {}
        }
        Action::None
    }

    fn handle_path_key(&mut self, code: KeyCode) -> Action {
        match code {
            // Esc は既存どおり保存して閉じる（`o` はパス文字として入力されるため対象外）。
            KeyCode::Esc => return self.save_or_close(),
            KeyCode::Up | KeyCode::Enter => self.focus = Focus::Options,
            KeyCode::Backspace => {
                self.path.pop();
            }
            KeyCode::Char(c) => self.path.push(c),
            _ => {}
        }
        Action::None
    }
}

impl Component for SettingsComponent {
    fn keymap(&self) -> &'static str {
        match self.focus {
            Focus::Options if self.is_specific_selected() => {
                "←→: Select  ↓: Edit path  o/Esc: Save & Close"
            }
            Focus::Options => "←→: Select  o/Esc: Save & Close",
            Focus::Path => "Type to edit path  ↑/Enter: Back  Esc: Save & Close",
        }
    }

    fn handle_event(&mut self, event: KeyEvent) -> Result<Action> {
        let action = match self.focus {
            Focus::Options => self.handle_options_key(event.code),
            Focus::Path => self.handle_path_key(event.code),
        };
        Ok(action)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let block = build_focused_block("Settings");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let label_style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);

        let mut spans: Vec<Span> = vec![Span::styled(" Startup Directory: ", label_style)];
        for (i, label) in StartupDirectory::LABELS.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw("  "));
            }
            let selected = i == self.selected_option;
            let marker = if selected { "[*]" } else { "[ ]" };
            let style = if selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            spans.push(Span::styled(format!("{marker} {label}"), style));
        }
        let mut lines = vec![Line::from(spans)];

        // Specific Directory 選択時のみパス入力フィールドを表示する。
        if self.is_specific_selected() {
            let mut path_spans: Vec<Span> = vec![
                Span::styled(" Path: ", label_style),
                Span::raw(self.path.clone()),
            ];
            // フォーカス中はカーソル（反転スペース）を末尾に表示する。
            if self.focus == Focus::Path {
                path_spans.push(Span::styled(
                    " ",
                    Style::default().add_modifier(Modifier::REVERSED),
                ));
            }
            lines.push(Line::from(path_spans));
        }

        frame.render_widget(Paragraph::new(lines), inner);
    }
}
