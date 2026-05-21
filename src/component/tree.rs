use crate::component::{Action, Component};
use crate::state::TreeState;
use crate::ui::widgets::{BorderStyle, build_bordered_block};
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
}

impl Component for TreeComponent {
    fn handle_event(&mut self, event: KeyEvent) -> Result<Action> {
        match event.code {
            KeyCode::Char('t') | KeyCode::Esc => Ok(Action::CloseSidePanel),
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
        let block = build_bordered_block(&title, BorderStyle::Active);

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
