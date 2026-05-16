use crate::component::{Action, Component};
use crate::state::{FileActionCandidateType, PromptMode};
use crate::ui::widgets::{BorderStyle, build_bordered_block};
use anyhow::{Context, Result};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::path::Path;
use unicode_width::UnicodeWidthChar;

use crate::fs::VFile;

pub struct PromptComponent {
    pub mode: PromptMode,
}

impl PromptComponent {
    pub fn new() -> Self {
        Self {
            mode: PromptMode::None,
        }
    }

    fn handle_input_event(&mut self, key: KeyEvent) -> Result<Action> {
        // 共通キー
        match key.code {
            KeyCode::Char(c) => return self.input_char(c),
            KeyCode::Backspace => return self.input_backspace(),
            KeyCode::Left => return self.input_cursor_left(),
            KeyCode::Right => return self.input_cursor_right(),
            KeyCode::Enter => return self.input_ok(),
            KeyCode::Esc => return Ok(Action::CancelPrompt),
            _ => {}
        }
        // モード固有キー
        match (&self.mode, key.code) {
            (PromptMode::File { .. }, KeyCode::Tab) => self.input_tab(),
            (PromptMode::File { .. }, KeyCode::BackTab) => self.input_back_tab(),
            (PromptMode::Search { value, .. }, KeyCode::Down) => {
                Ok(Action::SearchNext(value.clone()))
            }
            (PromptMode::Search { value, .. }, KeyCode::Up) => {
                Ok(Action::SearchPrev(value.clone()))
            }
            _ => Ok(Action::None),
        }
    }

    fn input_char(&mut self, c: char) -> Result<Action> {
        match &mut self.mode {
            PromptMode::Text { value, cursor, .. }
            | PromptMode::File { value, cursor, .. }
            | PromptMode::Search { value, cursor, .. } => {
                let byte_pos = char_to_byte_pos(value, *cursor);
                value.insert(byte_pos, c);
                *cursor += 1;
            }
            _ => {}
        }
        Ok(self.after_input_value_changed())
    }

    fn input_backspace(&mut self) -> Result<Action> {
        match &mut self.mode {
            PromptMode::Text { value, cursor, .. }
            | PromptMode::File { value, cursor, .. }
            | PromptMode::Search { value, cursor, .. } => {
                if *cursor > 0 {
                    *cursor -= 1;
                    let byte_pos = char_to_byte_pos(value, *cursor);
                    let next_byte_pos =
                        byte_pos + value[byte_pos..].chars().next().map_or(0, |c| c.len_utf8());
                    value.replace_range(byte_pos..next_byte_pos, "");
                }
            }
            _ => {}
        }
        Ok(self.after_input_value_changed())
    }

    fn input_cursor_left(&mut self) -> Result<Action> {
        match &mut self.mode {
            PromptMode::Text { cursor, .. }
            | PromptMode::File { cursor, .. }
            | PromptMode::Search { cursor, .. } => {
                *cursor = cursor.saturating_sub(1);
            }
            _ => {}
        }
        Ok(Action::None)
    }

    fn input_cursor_right(&mut self) -> Result<Action> {
        match &mut self.mode {
            PromptMode::Text { value, cursor, .. }
            | PromptMode::File { value, cursor, .. }
            | PromptMode::Search { value, cursor, .. } => {
                let char_count = value.chars().count();
                if *cursor < char_count {
                    *cursor += 1;
                }
            }
            _ => {}
        }
        Ok(Action::None)
    }

    fn input_tab(&mut self) -> Result<Action> {
        self.cycle_tab(CycleDirection::Forward)
    }

    fn input_back_tab(&mut self) -> Result<Action> {
        self.cycle_tab(CycleDirection::Backward)
    }

    fn cycle_tab(&mut self, direction: CycleDirection) -> Result<Action> {
        if let PromptMode::File {
            value,
            cursor,
            candidate_type,
            candidates,
            candidate_index,
            ..
        } = &mut self.mode
        {
            let compute = match candidate_type {
                FileActionCandidateType::All => compute_all_path_candidates,
                FileActionCandidateType::Directory => compute_dir_path_candidates,
            };
            cycle_candidates(value, candidates, candidate_index, direction, Some(compute))?;
            *cursor = value.chars().count();
        }
        Ok(Action::None)
    }

    fn input_ok(&mut self) -> Result<Action> {
        // Search モードでは Enter でモードを閉じるだけ
        if matches!(self.mode, PromptMode::Search { .. }) {
            self.mode = PromptMode::None;
            return Ok(Action::None);
        }
        let input = std::mem::replace(&mut self.mode, PromptMode::None);
        Ok(Action::ExecutePrompt(Box::new(input)))
    }

    fn handle_select_event(&mut self, key: KeyEvent) -> Result<Action> {
        match key.code {
            KeyCode::Left => {
                if let PromptMode::Select {
                    selected_index,
                    options,
                    ..
                } = &mut self.mode
                {
                    if *selected_index > 0 {
                        *selected_index -= 1;
                    } else {
                        *selected_index = options.len().saturating_sub(1);
                    }
                }
                Ok(Action::None)
            }
            KeyCode::Right => {
                if let PromptMode::Select {
                    selected_index,
                    options,
                    ..
                } = &mut self.mode
                {
                    if *selected_index + 1 < options.len() {
                        *selected_index += 1;
                    } else {
                        *selected_index = 0;
                    }
                }
                Ok(Action::None)
            }
            KeyCode::Enter => self.input_ok(),
            KeyCode::Esc => Ok(Action::CancelPrompt),
            _ => Ok(Action::None),
        }
    }

    fn handle_confirm_event(&mut self, key: KeyEvent) -> Result<Action> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Enter => self.input_ok(),
            KeyCode::Char('n') | KeyCode::Esc => Ok(Action::CancelPrompt),
            _ => Ok(Action::None),
        }
    }

    fn handle_error_event(&mut self, key: KeyEvent) -> Result<Action> {
        match key.code {
            KeyCode::Enter | KeyCode::Esc => Ok(Action::CancelPrompt),
            _ => Ok(Action::None),
        }
    }

    fn after_input_value_changed(&mut self) -> Action {
        self.mode.reset_candidates();
        if let PromptMode::Search { value, .. } = &self.mode {
            Action::SearchUpdate(value.clone())
        } else {
            Action::None
        }
    }

    fn render_prompt(&self, frame: &mut Frame, area: Rect) {
        let widget = match &self.mode {
            PromptMode::None => Paragraph::new("q: Quit")
                .block(build_bordered_block("Commands", BorderStyle::Inactive)),
            PromptMode::Text { title, value, .. }
            | PromptMode::File { title, value, .. }
            | PromptMode::Search { title, value, .. } => Paragraph::new(value.as_str())
                .block(build_bordered_block(title.as_ref(), BorderStyle::Active)),
            PromptMode::Select {
                title,
                options,
                selected_index,
                ..
            } => {
                let mut spans: Vec<Span> = Vec::new();
                for (i, opt) in options.iter().enumerate() {
                    if i > 0 {
                        spans.push(Span::raw(" "));
                    }
                    if i == *selected_index {
                        spans.push(Span::styled(
                            format!("[{opt}]"),
                            Style::default().add_modifier(Modifier::REVERSED),
                        ));
                    } else {
                        spans.push(Span::raw(format!(" {opt} ")));
                    }
                }
                Paragraph::new(Line::from(spans))
                    .block(build_bordered_block(title.as_str(), BorderStyle::Active))
            }
            PromptMode::Confirm { title, .. } => Paragraph::new("Yes(y) No(n)")
                .block(build_bordered_block(title.as_str(), BorderStyle::Active)),
            PromptMode::Error { message } => Paragraph::new(message.as_str())
                .style(Style::default().fg(Color::Red))
                .block(build_bordered_block("Error", BorderStyle::Error)),
        };
        frame.render_widget(widget, area);

        // テキスト入力時にカーソルを表示
        if let Some((cursor_pos, value)) = self.mode.cursor_and_value() {
            let display_width: usize = value
                .chars()
                .take(cursor_pos)
                .map(|c| UnicodeWidthChar::width(c).unwrap_or(0))
                .sum();
            // ボーダー(1) + パディング(1) + 表示幅
            let cursor_x = area.x + 2 + display_width as u16;
            let cursor_y = area.y + 1;
            frame.set_cursor_position(ratatui::layout::Position::new(cursor_x, cursor_y));
        }
    }
}

impl Component for PromptComponent {
    fn handle_event(&mut self, event: KeyEvent) -> Result<Action> {
        match &self.mode {
            PromptMode::Text { .. }
            | PromptMode::File { .. }
            | PromptMode::Search { .. } => self.handle_input_event(event),
            PromptMode::Select { .. } => self.handle_select_event(event),
            PromptMode::Confirm { .. } => self.handle_confirm_event(event),
            PromptMode::Error { .. } => self.handle_error_event(event),
            PromptMode::None => Ok(Action::None),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.render_prompt(frame, area);
    }
}

fn char_to_byte_pos(s: &str, char_index: usize) -> usize {
    s.char_indices()
        .nth(char_index)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

type ComputeCandidates = fn(&str) -> Result<Vec<String>>;

#[derive(Debug)]
enum CycleDirection {
    Forward,
    Backward,
}

fn cycle_candidates(
    value: &mut String,
    candidates: &mut Vec<String>,
    candidate_index: &mut Option<usize>,
    direction: CycleDirection,
    compute: Option<ComputeCandidates>,
) -> Result<()> {
    if candidates.is_empty() {
        if let Some(compute) = compute {
            *candidates = compute(value)?;
            if !candidates.is_empty() {
                let start = match direction {
                    CycleDirection::Forward => 0,
                    CycleDirection::Backward => candidates.len() - 1,
                };
                *candidate_index = Some(start);
                *value = candidates[start].clone();
            }
        }
    } else if let Some(index) = candidate_index {
        let next = match direction {
            CycleDirection::Forward => (*index + 1) % candidates.len(),
            CycleDirection::Backward => {
                if *index == 0 {
                    candidates.len() - 1
                } else {
                    *index - 1
                }
            }
        };
        *candidate_index = Some(next);
        *value = candidates[next].clone();
    }
    Ok(())
}

fn compute_all_path_candidates(input: &str) -> Result<Vec<String>> {
    compute_path_candidates(input, false)
}

fn compute_dir_path_candidates(input: &str) -> Result<Vec<String>> {
    compute_path_candidates(input, true)
}

fn compute_path_candidates(input: &str, dir_only: bool) -> Result<Vec<String>> {
    let path = Path::new(input);
    let (dir_path, prefix) = if input.ends_with('/') {
        (path.to_path_buf(), String::new())
    } else {
        let dir = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| p.to_path_buf())
            .context("Failed to get parent directory")?;
        let prefix = path
            .file_name()
            .context("Failed to get file name")?
            .to_string_lossy()
            .to_string();
        (dir, prefix)
    };

    let files = VFile::new(dir_path.to_string_lossy()).list()?;

    let mut candidates: Vec<String> = files
        .into_iter()
        .filter_map(|f| {
            if dir_only && !f.is_dir() {
                return None;
            }
            let name = f.file_name()?;
            if !name.starts_with(&prefix) {
                return None;
            }
            let mut s = f.absolute_path().to_string();
            if f.is_dir() {
                s.push('/');
            }
            Some(s)
        })
        .collect();

    candidates.sort();
    Ok(candidates)
}
