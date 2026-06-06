use crate::component::{Action, Component};
use crate::store::StartupDirectory;
use crate::ui::widgets::build_focused_block;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

/// Specific Directory 選択肢のインデックス（`StartupDirectory::LABELS` の末尾）。
const SPECIFIC_INDEX: usize = 3;

pub struct SettingsComponent {
    /// 初期値のインデックス
    initial_option: usize,
    /// 選択中のオプションインデックス
    selected_option: usize,
    /// Specific Directory のパス入力バッファ。選択肢を切り替えても保持し、戻したときに
    /// 入力中の内容が消えないようにする。保存済みの Specific パスが無い場合は
    /// 既定値として Filer のカレントディレクトリを入れておく。
    path: String,
    /// 初期パス（dirty 判定用）。
    initial_path: String,
}

impl SettingsComponent {
    pub fn new(startup_dir: &StartupDirectory, current_dir: &str) -> Self {
        let index = startup_dir.index();
        // 保存済みの Specific パスがあればそれを、無ければカレントディレクトリを既定値にする。
        let path = match startup_dir {
            StartupDirectory::SpecificDirectory(p) => p.clone(),
            _ => current_dir.to_string(),
        };
        Self {
            initial_option: index,
            selected_option: index,
            initial_path: path.clone(),
            path,
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
}

impl Component for SettingsComponent {
    fn keymap(&self) -> &'static str {
        if self.is_specific_selected() {
            // Specific 選択中は文字キーがパス入力に使われるため、保存は Enter / Esc。
            "←→: Select  Type: Edit path  Enter/Esc: Save & Close"
        } else {
            "←→: Select  Enter/o/Esc: Save & Close"
        }
    }

    fn handle_event(&mut self, event: KeyEvent) -> Result<Action> {
        let specific = self.is_specific_selected();
        let action = match event.code {
            // Specific 編集中は 'o' をパス文字として入力するため、保存は Enter / Esc で行う。
            KeyCode::Char('o') if !specific => self.save_or_close(),
            KeyCode::Enter | KeyCode::Esc => self.save_or_close(),
            KeyCode::Left => {
                self.selected_option = self.selected_option.saturating_sub(1);
                Action::None
            }
            KeyCode::Right if self.selected_option + 1 < StartupDirectory::LABELS.len() => {
                self.selected_option += 1;
                Action::None
            }
            // Specific 選択中はそのままパスを編集できる（フォーカス移動は不要）。
            KeyCode::Char(c) if specific => {
                self.path.push(c);
                Action::None
            }
            KeyCode::Backspace if specific => {
                self.path.pop();
                Action::None
            }
            _ => Action::None,
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

        // 横幅の狭いサイドパネルに収まるよう、選択肢は 1 行 1 項目の縦リストで表示する。
        let mut lines: Vec<Line> =
            vec![Line::from(Span::styled(" Startup Directory:", label_style))];
        for (i, label) in StartupDirectory::LABELS.iter().enumerate() {
            let selected = i == self.selected_option;
            let marker = if selected { "[*]" } else { "[ ]" };
            let style = if selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            lines.push(Line::from(Span::styled(
                format!("  {marker} {label}"),
                style,
            )));
        }

        // Specific Directory 選択時のみ、その直下にパス入力フィールドを表示する。
        // 選択中は常に編集可能なので、末尾にカーソル（反転スペース）を表示する。
        if self.is_specific_selected() {
            let path_spans: Vec<Span> = vec![
                Span::styled("    Path: ", label_style),
                Span::raw(self.path.clone()),
                Span::styled(" ", Style::default().add_modifier(Modifier::REVERSED)),
            ];
            lines.push(Line::from(path_spans));
        }

        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    }
}
