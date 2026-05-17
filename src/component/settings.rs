use crate::component::{Action, Component};
use crate::store::StartupDirectory;
use crate::ui::widgets::{BorderStyle, build_bordered_block};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

pub struct SettingsComponent {
    /// 初期値のインデックス
    initial_option: usize,
    /// 選択中のオプションインデックス
    selected_option: usize,
}

impl SettingsComponent {
    pub fn new(startup_dir: &StartupDirectory) -> Self {
        let index = startup_dir.index();
        Self {
            initial_option: index,
            selected_option: index,
        }
    }

    fn is_dirty(&self) -> bool {
        self.selected_option != self.initial_option
    }

    fn to_startup_directory(&self) -> StartupDirectory {
        StartupDirectory::ALL[self.selected_option].clone()
    }
}

impl Component for SettingsComponent {
    fn handle_event(&mut self, event: KeyEvent) -> Result<Action> {
        match event.code {
            KeyCode::Char('o') | KeyCode::Esc => {
                if self.is_dirty() {
                    Ok(Action::SaveSettings(Box::new(self.to_startup_directory())))
                } else {
                    Ok(Action::CloseSidePanel)
                }
            }
            KeyCode::Left => {
                if self.selected_option > 0 {
                    self.selected_option -= 1;
                }
                Ok(Action::None)
            }
            KeyCode::Right => {
                if self.selected_option + 1 < StartupDirectory::ALL.len() {
                    self.selected_option += 1;
                }
                Ok(Action::None)
            }
            _ => Ok(Action::None),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let block = build_bordered_block("Settings", BorderStyle::Active);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut spans: Vec<Span> = vec![Span::styled(
            " Startup Directory: ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )];
        for (i, option) in StartupDirectory::ALL.iter().enumerate() {
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
            spans.push(Span::styled(format!("{marker} {}", option.label()), style));
        }

        let lines = vec![Line::from(spans)];
        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }
}
