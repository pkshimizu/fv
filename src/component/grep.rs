use crate::component::{Action, Component};
use crate::state::PathListState;
use crate::ui::widgets::{BorderStyle, Spinner, build_bordered_block};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Cell, Row, Table};
use std::sync::mpsc::Receiver;

pub struct GrepComponent {
    state: PathListState,
    spinner: Spinner,
}

impl GrepComponent {
    pub fn new(rx: Receiver<String>) -> Self {
        Self {
            state: PathListState::new(Vec::new(), Some(rx)),
            spinner: Spinner::new(),
        }
    }
}

impl Component for GrepComponent {
    fn keymap(&self) -> &'static str {
        "↑↓: Move  ←→: Top/Bottom  Enter: Open  g/Esc: Close"
    }

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

    fn tick(&mut self) {
        self.spinner.advance();
        self.state.receive_results();
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let is_running = self.state.is_running();
        let title = if is_running {
            format!(
                "Grep ({}) {}",
                self.state.paths.len(),
                self.spinner.label("Running")
            )
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::sync::mpsc;

    fn render_to_string(grep: &mut GrepComponent) -> String {
        let mut terminal = Terminal::new(TestBackend::new(80, 6)).expect("build test terminal");
        terminal
            .draw(|frame| {
                let area = frame.area();
                grep.render(frame, area);
            })
            .expect("draw grep");
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn running_grep_shows_a_spinner_in_the_title() {
        let (_tx, rx) = mpsc::channel::<String>();
        let mut grep = GrepComponent::new(rx);

        let text = render_to_string(&mut grep);

        assert!(
            text.contains('⠋'),
            "spinner expected while running, got: {text:?}"
        );
        assert!(
            text.contains("Running"),
            "running label expected, got: {text:?}"
        );
    }
}
