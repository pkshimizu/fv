use crate::component::{Action, Component};
use crate::state::PathListState;
use crate::ui::widgets::{BorderState, Focus, build_bordered_block};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Cell, Row, Table};

pub struct BookmarkComponent {
    state: PathListState,
}

impl BookmarkComponent {
    pub fn new(paths: Vec<String>) -> Self {
        Self {
            state: PathListState::new(paths, None),
        }
    }
}

impl Component for BookmarkComponent {
    fn keymap(&self) -> &'static str {
        "↑↓: Move  ←→: Top/Bottom  Enter: Open  -: Remove  b/Esc: Close"
    }

    fn handle_event(&mut self, event: KeyEvent) -> Result<Action> {
        match event.code {
            KeyCode::Char('b') | KeyCode::Esc => Ok(Action::CloseSidePanel),
            KeyCode::Up => {
                self.state.prev();
                Ok(Action::None)
            }
            KeyCode::Down => {
                self.state.next();
                Ok(Action::None)
            }
            KeyCode::Left => {
                self.state.first();
                Ok(Action::None)
            }
            KeyCode::Right => {
                self.state.last();
                Ok(Action::None)
            }
            KeyCode::Enter => {
                if let Some(path) = self.state.selected_path() {
                    Ok(Action::NavigateTo(path.to_string()))
                } else {
                    Ok(Action::None)
                }
            }
            KeyCode::Char('-') => {
                if let Some(path) = self.state.selected_path() {
                    let path = path.to_string();
                    self.state.remove(&path);
                    Ok(Action::RemoveBookmark(path))
                } else {
                    Ok(Action::None)
                }
            }
            _ => Ok(Action::None),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let title = format!("Bookmark ({})", self.state.paths.len());
        let block = build_bordered_block(&title, Focus::Focused, BorderState::Normal);
        let rows = self
            .state
            .paths
            .iter()
            .map(|path| Row::new([Cell::from(path.as_str())]));
        let table = Table::new(rows, [Constraint::Fill(1)])
            .block(block)
            .highlight_symbol("> ")
            .row_highlight_style(Style::default().add_modifier(Modifier::UNDERLINED));
        frame.render_stateful_widget(table, area, &mut self.state.table_state);
    }
}
