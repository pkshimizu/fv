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

/// 設定項目メニューに並ぶ項目ラベル（表示順）。今は "Startup Directory" のみ。
/// 項目を増やすときはここにラベルを足し、`enter_editor` に対応する遷移を加える。
const MENU_ITEMS: &[&str] = &["Startup Directory"];

/// Settings の内部 View。`SidePanel::Settings` のまま、項目メニューと項目エディタを
/// 行き来する（パネル自体は開いたまま）。
enum SettingsView {
    /// 設定項目を選ぶメニュー。
    Menu,
    /// Startup Directory の編集 UI。
    StartupDirectory(StartupDirectoryEditor),
}

/// Startup Directory 項目の編集状態。ラジオ選択（4択）と Specific のパス入力を持つ。
/// 項目を増やすときは、項目ごとにこの種のエディタ状態を `SettingsView` に足す。
struct StartupDirectoryEditor {
    /// 選択中のオプションインデックス（`StartupDirectory::LABELS` 上の位置）。
    selected_option: usize,
    /// Specific Directory のパス入力バッファ。選択肢を切り替えても保持し、戻したときに
    /// 入力中の内容が消えないようにする。保存済みの Specific パスが無い場合は
    /// 既定値として Filer のカレントディレクトリを入れておく。
    path: String,
}

/// エディタへキーを渡した結果。コンポーネントの View 遷移・保存判断に使う。
enum EditorOutcome {
    /// 編集を継続（View は変えない）。
    Continue,
    /// 確定（Enter）。最新の選択値で保存し、メニューへ戻る。
    Commit,
    /// 破棄（Esc）。保存せずメニューへ戻る。
    Cancel,
}

impl StartupDirectoryEditor {
    /// 現在の保存値（とカレントディレクトリ）から初期化する。
    fn new(startup_dir: &StartupDirectory, current_dir: &str) -> Self {
        let index = startup_dir.index();
        // 保存済みの Specific パスがあればそれを、無ければカレントディレクトリを既定値にする。
        let path = match startup_dir {
            StartupDirectory::SpecificDirectory(p) => p.clone(),
            _ => current_dir.to_string(),
        };
        Self {
            selected_option: index,
            path,
        }
    }

    fn is_specific_selected(&self) -> bool {
        self.selected_option == StartupDirectory::SPECIFIC_INDEX
    }

    fn to_startup_directory(&self) -> StartupDirectory {
        // 並び順 → 値の対応は store 側 `from_index` に一元化している。
        StartupDirectory::from_index(self.selected_option, &self.path)
    }

    fn handle_event(&mut self, event: KeyEvent) -> EditorOutcome {
        let specific = self.is_specific_selected();
        match event.code {
            KeyCode::Enter => return EditorOutcome::Commit,
            KeyCode::Esc => return EditorOutcome::Cancel,
            // 選択肢は縦リスト表示なので、上下キーで選択を移動する。
            KeyCode::Up => {
                self.selected_option = self.selected_option.saturating_sub(1);
            }
            KeyCode::Down if self.selected_option + 1 < StartupDirectory::LABELS.len() => {
                self.selected_option += 1;
            }
            // Specific 選択中はそのままパスを編集できる（フォーカス移動は不要）。
            // 制御文字（改行・タブ等）はパスに混入させない。`o` もここで文字として扱われ、
            // 閉じる操作には使われない（誤爆防止）。
            KeyCode::Char(c) if specific && !c.is_control() => {
                self.path.push(c);
            }
            KeyCode::Backspace if specific => {
                self.path.pop();
            }
            _ => {}
        }
        EditorOutcome::Continue
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
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
                Span::raw(self.path.as_str()),
                Span::styled(" ", Style::default().add_modifier(Modifier::REVERSED)),
            ];
            lines.push(Line::from(path_spans));
        }

        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }
}

pub struct SettingsComponent {
    /// 現在の View（項目メニュー / 項目エディタ）。
    view: SettingsView,
    /// メニューで選択中の項目インデックス（`MENU_ITEMS` 上の位置）。
    menu_index: usize,
    /// 現在保存されている起動ディレクトリ設定。エディタの初期値の元であり、保存のたびに
    /// 更新する。これにより、再度エディタへ入ったとき保存値が初期表示される。
    startup_dir: StartupDirectory,
    /// Filer のカレントディレクトリ（Specific 未設定時の既定パスに使う）。
    current_dir: String,
}

impl SettingsComponent {
    pub fn new(startup_dir: &StartupDirectory, current_dir: &str) -> Self {
        Self {
            view: SettingsView::Menu,
            menu_index: 0,
            startup_dir: startup_dir.clone(),
            current_dir: current_dir.to_string(),
        }
    }

    /// メニューで選択中の項目の編集 View へ遷移する。
    fn enter_editor(&mut self) {
        // 今は項目が Startup Directory のみなので menu_index は参照しない。
        // 項目が増えたら、ここで menu_index により生成するエディタを分岐する。
        self.view = SettingsView::StartupDirectory(StartupDirectoryEditor::new(
            &self.startup_dir,
            &self.current_dir,
        ));
    }

    fn handle_menu_event(&mut self, event: KeyEvent) -> Action {
        match event.code {
            KeyCode::Up => {
                self.menu_index = self.menu_index.saturating_sub(1);
                Action::None
            }
            KeyCode::Down if self.menu_index + 1 < MENU_ITEMS.len() => {
                self.menu_index += 1;
                Action::None
            }
            KeyCode::Enter => {
                self.enter_editor();
                Action::None
            }
            KeyCode::Char('o') | KeyCode::Esc => Action::CloseSidePanel,
            _ => Action::None,
        }
    }

    fn render_menu(&self, frame: &mut Frame, area: Rect) {
        let label_style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);
        let mut lines: Vec<Line> = vec![Line::from(Span::styled(" Settings:", label_style))];
        for (i, label) in MENU_ITEMS.iter().enumerate() {
            let style = if i == self.menu_index {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            lines.push(Line::from(Span::styled(format!("  {label}"), style)));
        }
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }
}

impl Component for SettingsComponent {
    fn keymap(&self) -> &'static str {
        match &self.view {
            SettingsView::Menu => "↑↓: Select  Enter: Edit  o/Esc: Close",
            SettingsView::StartupDirectory(editor) if editor.is_specific_selected() => {
                // Specific 選択中は文字キーがパス入力に使われる。
                "↑↓: Select  Type: Edit path  Enter: Save  Esc: Cancel"
            }
            SettingsView::StartupDirectory(_) => "↑↓: Select  Enter: Save  Esc: Cancel",
        }
    }

    fn handle_event(&mut self, event: KeyEvent) -> Result<Action> {
        let action = match &mut self.view {
            SettingsView::Menu => self.handle_menu_event(event),
            SettingsView::StartupDirectory(editor) => match editor.handle_event(event) {
                EditorOutcome::Continue => Action::None,
                EditorOutcome::Cancel => {
                    // 破棄してメニューへ戻る（ドラフトはエディタごと捨てられる）。
                    self.view = SettingsView::Menu;
                    Action::None
                }
                EditorOutcome::Commit => {
                    let new = editor.to_startup_directory();
                    self.view = SettingsView::Menu;
                    // 変更があるときだけ永続化する。baseline を更新し、再入時に保存値を出す。
                    if new != self.startup_dir {
                        self.startup_dir = new.clone();
                        Action::SaveSettings(Box::new(new))
                    } else {
                        Action::None
                    }
                }
            },
        };
        Ok(action)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let block = build_focused_block("Settings");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        match &self.view {
            SettingsView::Menu => self.render_menu(frame, inner),
            SettingsView::StartupDirectory(editor) => editor.render(frame, inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn component() -> SettingsComponent {
        SettingsComponent::new(&StartupDirectory::CurrentDirectory, "/home/user")
    }

    #[test]
    fn enter_in_menu_switches_to_editor() {
        let mut c = component();
        assert!(matches!(c.view, SettingsView::Menu));
        let action = c.handle_event(key(KeyCode::Enter)).unwrap();
        assert!(matches!(action, Action::None));
        assert!(matches!(c.view, SettingsView::StartupDirectory(_)));
    }

    #[test]
    fn commit_after_change_saves_and_returns_to_menu() {
        let mut c = component();
        c.handle_event(key(KeyCode::Enter)).unwrap(); // メニュー → エディタ
        c.handle_event(key(KeyCode::Down)).unwrap(); // Current → Home に変更
        let action = c.handle_event(key(KeyCode::Enter)).unwrap();
        match action {
            Action::SaveSettings(dir) => {
                assert_eq!(*dir, StartupDirectory::HomeDirectory);
            }
            _ => panic!("expected SaveSettings"),
        }
        // メニューへ戻り、baseline が更新されている。
        assert!(matches!(c.view, SettingsView::Menu));
        assert_eq!(c.startup_dir, StartupDirectory::HomeDirectory);
    }

    #[test]
    fn commit_without_change_returns_to_menu_without_saving() {
        let mut c = component();
        c.handle_event(key(KeyCode::Enter)).unwrap(); // メニュー → エディタ
        let action = c.handle_event(key(KeyCode::Enter)).unwrap(); // 変更なしで確定
        assert!(matches!(action, Action::None));
        assert!(matches!(c.view, SettingsView::Menu));
    }

    #[test]
    fn esc_in_editor_discards_and_returns_to_menu() {
        let mut c = component();
        c.handle_event(key(KeyCode::Enter)).unwrap(); // メニュー → エディタ
        c.handle_event(key(KeyCode::Down)).unwrap(); // 変更（破棄される）
        let action = c.handle_event(key(KeyCode::Esc)).unwrap();
        assert!(matches!(action, Action::None));
        assert!(matches!(c.view, SettingsView::Menu));
        // baseline は変わらない（破棄されたため）。
        assert_eq!(c.startup_dir, StartupDirectory::CurrentDirectory);
    }

    #[test]
    fn esc_in_menu_closes_panel() {
        let mut c = component();
        let action = c.handle_event(key(KeyCode::Esc)).unwrap();
        assert!(matches!(action, Action::CloseSidePanel));
    }

    #[test]
    fn o_in_menu_closes_panel() {
        let mut c = component();
        let action = c.handle_event(key(KeyCode::Char('o'))).unwrap();
        assert!(matches!(action, Action::CloseSidePanel));
    }

    #[test]
    fn o_while_editing_specific_path_is_typed_not_close() {
        let mut c = component();
        c.handle_event(key(KeyCode::Enter)).unwrap(); // メニュー → エディタ
        // Specific を選択（Current(0) → Specific(3) まで Down 3 回）。
        for _ in 0..StartupDirectory::SPECIFIC_INDEX {
            c.handle_event(key(KeyCode::Down)).unwrap();
        }
        let action = c.handle_event(key(KeyCode::Char('o'))).unwrap();
        // 閉じず、エディタに留まる。
        assert!(matches!(action, Action::None));
        assert!(matches!(c.view, SettingsView::StartupDirectory(_)));
        if let SettingsView::StartupDirectory(editor) = &c.view {
            assert!(editor.path.ends_with('o'));
        }
    }

    #[test]
    fn reentering_editor_shows_saved_value() {
        let mut c = component();
        c.handle_event(key(KeyCode::Enter)).unwrap(); // メニュー → エディタ
        c.handle_event(key(KeyCode::Down)).unwrap(); // Current → Home
        c.handle_event(key(KeyCode::Enter)).unwrap(); // 保存してメニューへ
        // 再度エディタへ入ると保存値（Home, index 1）が初期選択になっている。
        c.handle_event(key(KeyCode::Enter)).unwrap();
        if let SettingsView::StartupDirectory(editor) = &c.view {
            assert_eq!(
                editor.selected_option,
                StartupDirectory::HomeDirectory.index()
            );
        } else {
            panic!("expected editor view");
        }
    }
}
