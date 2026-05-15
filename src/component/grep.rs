use crate::component::{Action, Component};
use crate::state::PathListState;
use crate::ui::widgets::{BorderStyle, build_bordered_block};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Cell, Row, Table};
use std::sync::mpsc::Receiver;

pub struct GrepComponent {
    state: PathListState,
}

impl GrepComponent {
    pub fn new(rx: Receiver<String>) -> Self {
        Self {
            state: PathListState::new(Vec::new(), Some(rx)),
        }
    }

    /// 非同期の grep 結果を受信する
    pub fn receive_results(&mut self) {
        self.state.receive_results();
    }
}

impl Component for GrepComponent {
    fn handle_event(&mut self, event: KeyEvent) -> Result<Action> {
        match event.code {
            KeyCode::Char('g') | KeyCode::Esc => Ok(Action::CloseSidePanel),
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
            _ => Ok(Action::None),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let is_running = self.state.is_running();
        let title = if is_running {
            format!("Grep ({}) Running", self.state.paths.len())
        } else {
            format!("Grep ({})", self.state.paths.len())
        };
        let block = build_bordered_block(&title, BorderStyle::Active);
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
