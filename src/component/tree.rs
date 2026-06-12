use crate::component::{Action, Component};
use crate::state::{PromptMode, TreeState};
use crate::ui::widgets::build_focused_block;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Cell, Row, Table};

const DIR_STYLE: Style = Style::new().fg(Color::Green);
const DOTFILE_STYLE: Style = Style::new().fg(Color::Blue);

pub struct TreeComponent {
    state: TreeState,
}

impl TreeComponent {
    pub fn new(current_path: &str, show_dot_file: bool) -> Self {
        Self {
            state: TreeState::new(current_path, show_dot_file),
        }
    }

    /// 表示中のエントリからクエリにマッチする最初のものへカーソルを移動する。
    pub fn select_matching(&mut self, query: &str) {
        self.state.select_matching(query);
    }

    /// 次のマッチへ移動する。
    pub fn select_next_matching(&mut self, query: &str) {
        self.state.select_next_matching(query);
    }

    /// 前のマッチへ移動する。
    pub fn select_prev_matching(&mut self, query: &str) {
        self.state.select_prev_matching(query);
    }

    /// カーソル位置を直接設定する（Search の Esc 復元で使う）。
    pub fn select_index(&mut self, index: Option<usize>) {
        self.state.select_index(index);
    }
}

impl Component for TreeComponent {
    fn keymap(&self) -> &'static str {
        "↑↓: Move  →: Expand  ←: Collapse  f: Search  Enter: Open  t/Esc: Close"
    }

    fn handle_event(&mut self, event: KeyEvent) -> Result<Action> {
        match event.code {
            KeyCode::Char('t') | KeyCode::Esc => Ok(Action::CloseSidePanel),
            // f でファイル名検索を開始する。Filer の Search と同じ仕組み（PromptMode::Search）を
            // 再利用し、App 側が「ツリーパネルが開いているか」で検索対象を振り分ける。
            KeyCode::Char('f') => Ok(Action::SetPromptMode(Box::new(PromptMode::Search {
                title: "Search".to_string(),
                value: String::new(),
                cursor: 0,
                original_index: self.state.selected_index(),
            }))),
            KeyCode::Up => {
                self.state.prev();
                Ok(Action::None)
            }
            KeyCode::Down => {
                self.state.next();
                Ok(Action::None)
            }
            KeyCode::Right => {
                self.state.expand_selected();
                Ok(Action::None)
            }
            KeyCode::Left => {
                self.state.collapse_selected();
                Ok(Action::None)
            }
            KeyCode::Enter => {
                if let Some(path) = self.state.selected_path() {
                    Ok(Action::NavigateTo(path.to_string()))
                } else {
                    Ok(Action::None)
                }
            }
            _ => Ok(Action::None),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let title = format!("Tree ({})", self.state.flat_nodes.len());
        let block = build_focused_block(&title);

        let rows = self.state.flat_nodes.iter().map(|entry| {
            let indent = "  ".repeat(entry.depth);
            let icon = if entry.is_dir {
                if entry.expanded { "- " } else { "+ " }
            } else {
                "  "
            };
            let display = format!("{indent}{icon}{}", entry.name);
            let style = if entry.is_dir {
                DIR_STYLE
            } else if entry.name.starts_with('.') {
                DOTFILE_STYLE
            } else {
                Style::default()
            };
            Row::new([Cell::from(display)]).style(style)
        });

        let table = Table::new(rows, [Constraint::Fill(1)])
            .block(block)
            .highlight_symbol("> ")
            .row_highlight_style(Style::default().add_modifier(Modifier::UNDERLINED));
        frame.render_stateful_widget(table, area, &mut self.state.table_state);
    }
}
