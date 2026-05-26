use crate::app_context::AppContext;
use crate::component::{Action, Component, GrepComponent};
use crate::state::{
    ConfirmAction, FileAction, FileActionCandidateType, ProgressMessage, PromptMode, SelectAction,
    SidePanel, SortKey, TextAction,
};
use crate::store::RootStore;
use crate::ui::widgets::{BorderStyle, build_bordered_block};
use anyhow::{Context, Result};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::io::BufRead;
use std::path::Path;
use std::sync::mpsc;
use unicode_width::UnicodeWidthChar;

use crate::fs::{CopyProgress, VFile, copy_files_with_progress};

pub struct PromptComponent {
    mode: PromptMode,
    progress: Option<mpsc::Receiver<ProgressMessage>>,
}

impl PromptComponent {
    pub fn new() -> Self {
        Self {
            mode: PromptMode::None,
            progress: None,
        }
    }

    pub fn is_active(&self) -> bool {
        self.mode.is_active()
    }

    pub fn set_mode(&mut self, mode: PromptMode) {
        self.mode = mode;
    }

    pub fn set_error(&mut self, message: String) {
        self.mode = PromptMode::Error { message };
    }

    /// 非同期処理の進捗表示を開始する。
    /// receiver から ProgressMessage を受信し、promptエリアに進捗を表示する。
    pub fn start_progress(&mut self, message: String, receiver: mpsc::Receiver<ProgressMessage>) {
        self.mode = PromptMode::Progress { message };
        self.progress = Some(receiver);
    }

    pub fn cancel(&mut self) -> Option<usize> {
        let original_index = if let PromptMode::Search { original_index, .. } = &self.mode {
            *original_index
        } else {
            None
        };
        self.mode = PromptMode::None;
        original_index
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
        // Execute アクションはターミナル制御が必要なため、専用の Action に変換する
        if let PromptMode::Text {
            action: ref a,
            ref value,
            ..
        } = input
            && let TextAction::Execute { ref dir } = **a
        {
            return Ok(Action::ExecuteCommand(
                value.clone(),
                dir.absolute_path().to_string(),
            ));
        }
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
            PromptMode::Progress { message } => Paragraph::new(message.as_str())
                .block(build_bordered_block("Progress", BorderStyle::Active)),
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
            PromptMode::Text { .. } | PromptMode::File { .. } | PromptMode::Search { .. } => {
                self.handle_input_event(event)
            }
            PromptMode::Select { .. } => self.handle_select_event(event),
            PromptMode::Confirm { .. } => self.handle_confirm_event(event),
            PromptMode::Error { .. } => self.handle_error_event(event),
            PromptMode::Progress { .. } => Ok(Action::None),
            PromptMode::None => Ok(Action::None),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.render_prompt(frame, area);
    }

    fn tick(&mut self) {
        // 溜まったメッセージを全て消費し、最新の進捗状態のみを反映する
        let Some(receiver) = self.progress.as_ref() else {
            return;
        };
        loop {
            match receiver.try_recv() {
                Ok(ProgressMessage::Update(text)) => {
                    self.mode = PromptMode::Progress { message: text };
                }
                Ok(ProgressMessage::Complete) => {
                    self.mode = PromptMode::None;
                    self.progress = None;
                    return;
                }
                Ok(ProgressMessage::Error(text)) => {
                    self.mode = PromptMode::Error { message: text };
                    self.progress = None;
                    return;
                }
                Err(mpsc::TryRecvError::Empty) => return,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.mode = PromptMode::Error {
                        message: "Progress channel disconnected unexpectedly".to_string(),
                    };
                    self.progress = None;
                    return;
                }
            }
        }
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

// --- プロンプト確定後のアクション実行 ---

/// プロンプトの確定アクションを実行する。
/// PromptComponent の input_ok が Action::ExecutePrompt(PromptMode) を返し、
/// App::handle_action がこの関数を呼び出す。
pub fn execute_prompt_action(
    ctx: &mut AppContext,
    store: &mut RootStore,
    input: PromptMode,
) -> Result<()> {
    let skip_clear = matches!(
        input,
        PromptMode::Select {
            action: SelectAction::Sort,
            ..
        }
    );
    match input {
        PromptMode::Confirm { action, .. } => execute_confirm_action(action),
        PromptMode::Text { action, value, .. } => {
            execute_text_action(ctx, store, *action, value.as_str())
        }
        PromptMode::File { action, value, .. } => execute_file_action(ctx, action, value.as_str()),
        PromptMode::Select {
            action,
            selected_index,
            ..
        } => execute_select_action(ctx, action, selected_index),
        PromptMode::None
        | PromptMode::Error { .. }
        | PromptMode::Search { .. }
        | PromptMode::Progress { .. } => Ok(()),
    }?;
    if !skip_clear {
        ctx.filer.clear_checked_paths();
    }
    Ok(())
}

fn execute_confirm_action(action: ConfirmAction) -> Result<()> {
    match action {
        ConfirmAction::Delete { files } => {
            for file in &files {
                file.delete()?;
            }
            Ok(())
        }
    }
}

fn execute_text_action(
    ctx: &mut AppContext,
    store: &mut RootStore,
    action: TextAction,
    value: &str,
) -> Result<()> {
    match action {
        TextAction::Mkdir { dir } => dir.create_dir(value),
        TextAction::Touch { dir } => dir.create_file(value),
        TextAction::Rename { file } => {
            file.rename(value)?;
            ctx.filer.set_pending_select_name(value.to_string());
            Ok(())
        }
        TextAction::Zip { dir, files } => dir.create_zip(value, &files),
        TextAction::Unzip { file, dir } => {
            anyhow::ensure!(
                std::path::Path::new(value)
                    .components()
                    .all(|c| matches!(c, std::path::Component::Normal(_))),
                "{value}: Invalid directory name"
            );
            let dest_path = std::path::Path::new(dir.absolute_path()).join(value);
            std::fs::create_dir_all(&dest_path)
                .with_context(|| format!("{}: Failed to create directory", dest_path.display()))?;
            let dest_str = dest_path.to_str().context("Invalid path")?;
            file.extract_zip(dest_str)?;
            Ok(())
        }
        TextAction::Grep => execute_grep(ctx, store, value),
        // input_ok で Action::ExecuteCommand に変換されるため到達しない
        TextAction::Execute { .. } => unreachable!(),
    }
}

fn execute_file_action(ctx: &mut AppContext, action: FileAction, value: &str) -> Result<()> {
    match action {
        FileAction::Copy { files } => {
            start_async_copy(ctx, files, value.to_string());
            Ok(())
        }
        FileAction::Move { files } => {
            for file in &files {
                file.move_to(value)?;
            }
            Ok(())
        }
        FileAction::Jump => {
            let path = Path::new(value);
            anyhow::ensure!(path.is_dir(), "{value} はディレクトリではありません");
            ctx.filer.change_to(value);
            Ok(())
        }
    }
}

fn execute_select_action(
    ctx: &mut AppContext,
    action: SelectAction,
    selected_index: usize,
) -> Result<()> {
    match action {
        SelectAction::Sort => {
            if let Some(&sort_key) = SortKey::ALL.get(selected_index) {
                ctx.filer.set_sort_key(sort_key);
                ctx.filer.refresh_files();
            }
            Ok(())
        }
    }
}

fn execute_grep(ctx: &mut AppContext, _store: &mut RootStore, value: &str) -> Result<()> {
    if value.is_empty() {
        return Ok(());
    }

    let dir_path = ctx.filer.current_dir_path().to_string();
    let pattern = value.to_string();

    let mut child = std::process::Command::new("grep")
        .args([
            "-rlF",
            "--binary-files=without-match",
            "--",
            &pattern,
            &dir_path,
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("Failed to execute grep")?;

    let stdout = child.stdout.take().context("Failed to take stdout")?;

    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stdout);
        let mut canceled = false;
        for line in reader.lines() {
            let Ok(path) = line else { break };
            if tx.send(path).is_err() {
                canceled = true;
                break;
            }
        }
        if canceled {
            let _ = child.kill();
        }
        let _ = child.wait();
    });

    ctx.side_panel = Some(SidePanel::Grep(GrepComponent::new(rx)));
    Ok(())
}

fn start_async_copy(ctx: &mut AppContext, files: Vec<VFile>, dest: String) {
    use std::sync::atomic::{AtomicBool, Ordering};

    let (tx, rx) = mpsc::channel();
    ctx.prompt.start_progress("Copying...".to_string(), rx);

    std::thread::spawn(move || {
        let cancel = AtomicBool::new(false);
        let result = copy_files_with_progress(&files, &dest, &cancel, |progress| {
            if tx
                .send(ProgressMessage::Update(format_copy_progress(&progress)))
                .is_err()
            {
                // 受信側が消えた。コピーを早期中断する。
                cancel.store(true, Ordering::Relaxed);
            }
        });
        match result {
            Ok(()) => {
                let _ = tx.send(ProgressMessage::Complete);
            }
            Err(e) => {
                let _ = tx.send(ProgressMessage::Error(format!("{e:#}")));
            }
        }
    });
}

fn format_copy_progress(progress: &CopyProgress) -> String {
    let total = match progress.total_files {
        Some(n) => n.to_string(),
        None => "?".to_string(),
    };
    format!(
        "Copying {}/{} files  {} / {}",
        progress.copied_files,
        total,
        format_bytes(progress.current_bytes),
        format_bytes(progress.current_total_bytes),
    )
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}
