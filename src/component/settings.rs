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

/// StartupDirectory の選択肢
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartupDirOption {
    Current,
    Home,
}

impl StartupDirOption {
    const ALL: &'static [StartupDirOption] = &[StartupDirOption::Current, StartupDirOption::Home];

    fn label(&self) -> &'static str {
        match self {
            StartupDirOption::Current => "Current Directory",
            StartupDirOption::Home => "Home Directory",
        }
    }

    fn from_startup_directory(dir: &StartupDirectory) -> usize {
        match dir {
            StartupDirectory::CurrentDirectory => 0,
            StartupDirectory::HomeDirectory => 1,
        }
    }

    fn to_startup_directory(index: usize) -> StartupDirectory {
        match Self::ALL[index] {
            StartupDirOption::Current => StartupDirectory::CurrentDirectory,
            StartupDirOption::Home => StartupDirectory::HomeDirectory,
        }
    }
}

pub struct SettingsComponent {
    /// 選択中のオプション
    selected_option: usize,
    /// 変更があったかどうか
    dirty: bool,
}

impl SettingsComponent {
    pub fn new(startup_dir: &StartupDirectory) -> Self {
        Self {
            selected_option: StartupDirOption::from_startup_directory(startup_dir),
            dirty: false,
        }
    }

    fn to_startup_directory(&self) -> StartupDirectory {
        StartupDirOption::to_startup_directory(self.selected_option)
    }
}

impl Component for SettingsComponent {
    fn handle_event(&mut self, event: KeyEvent) -> Result<Action> {
        match event.code {
            KeyCode::Char('o') | KeyCode::Esc => {
                if self.dirty {
                    Ok(Action::SaveSettings(Box::new(self.to_startup_directory())))
                } else {
                    Ok(Action::CloseSidePanel)
                }
            }
            KeyCode::Left => {
                if self.selected_option > 0 {
                    self.selected_option -= 1;
                    self.dirty = true;
                }
                Ok(Action::None)
            }
            KeyCode::Right => {
                if self.selected_option + 1 < StartupDirOption::ALL.len() {
                    self.selected_option += 1;
                    self.dirty = true;
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

        let mut lines: Vec<Line> = Vec::new();

        // 設定項目: Startup Directory（ラベル + 横並びラジオボタン）
        let mut spans: Vec<Span> = vec![Span::styled(
            " Startup Directory: ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )];
        for (i, option) in StartupDirOption::ALL.iter().enumerate() {
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
        lines.push(Line::from(spans));

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }
}
