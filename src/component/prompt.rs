use crate::app::async_job::{AsyncJobHandle, spawn_async_job};
use crate::app_context::AppContext;
use crate::component::{Action, Component, GrepComponent};
use crate::fs::VFile;
use crate::fs::async_job::FileJob;
use crate::state::{
    ConfirmAction, FileAction, FileActionCandidateType, Phase, ProgressMessage, PromptMode,
    SelectAction, SidePanel, SortKey, TextAction,
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
use std::fmt::Write as _;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use unicode_width::UnicodeWidthChar;

pub struct PromptComponent {
    mode: PromptMode,
    handle: Option<AsyncJobHandle>,
    /// PromptMode::Progress 表示用の文字列バッファ。
    /// 毎フレーム `format!` で再確保しないよう、進捗状態が変わったタイミングのみ
    /// `clear() + write!` で詰め替える。
    progress_buf: String,
}

impl PromptComponent {
    pub fn new() -> Self {
        Self {
            mode: PromptMode::None,
            handle: None,
            progress_buf: String::new(),
        }
    }

    pub fn is_active(&self) -> bool {
        self.mode.is_active()
    }

    pub fn set_mode(&mut self, mode: PromptMode) {
        self.mode = mode;
        self.refresh_progress_text();
    }

    pub fn set_error(&mut self, message: String) {
        self.mode = PromptMode::Error { message };
        self.refresh_progress_text();
    }

    /// Async Job の進捗表示を開始する。受信した Update メッセージを `tick()` が PromptMode::Progress に反映する。
    pub fn start_async_job(&mut self, handle: AsyncJobHandle, initial_phase: Phase) {
        self.mode = PromptMode::Progress {
            phase: initial_phase,
            processed: 0,
            total: None,
        };
        self.handle = Some(handle);
        self.refresh_progress_text();
    }

    /// 現在 Async Job が実行中か。
    pub fn is_job_running(&self) -> bool {
        self.handle.is_some()
    }

    /// Async Job からの Update を PromptMode::Progress に反映する。
    /// `Cancelling` phase は sticky で、worker からの後続 phase で上書きされない。
    fn apply_progress(&mut self, phase: Phase, processed: usize, total: Option<usize>) {
        match &mut self.mode {
            PromptMode::Progress {
                phase: cur_phase,
                processed: cur_processed,
                total: cur_total,
            } => {
                if *cur_phase != Phase::Cancelling {
                    *cur_phase = phase;
                }
                *cur_processed = processed;
                *cur_total = total;
            }
            _ => {
                self.mode = PromptMode::Progress {
                    phase,
                    processed,
                    total,
                };
            }
        }
        self.refresh_progress_text();
    }

    /// Esc 受信時の処理: Cancel Token を立て、表示フェーズを Cancelling に切り替える。
    /// worker は次の File-level Checkpoint で停止する。
    fn request_cancel(&mut self) {
        if let Some(handle) = self.handle.as_ref() {
            handle.cancel.store(true, Ordering::Relaxed);
        }
        if let PromptMode::Progress { phase, .. } = &mut self.mode {
            *phase = Phase::Cancelling;
        }
        self.refresh_progress_text();
    }

    /// Async Job が終端メッセージで終わったとき、ハンドルを解放して新しい mode に遷移する。
    fn clear_job(&mut self, new_mode: PromptMode) {
        self.handle = None;
        self.mode = new_mode;
        self.refresh_progress_text();
    }

    /// 現在の `PromptMode::Progress` の値に基づいて `progress_buf` を更新する。
    /// 他のモードのときはバッファをクリアする。
    fn refresh_progress_text(&mut self) {
        self.progress_buf.clear();
        let (phase, processed, total) = match &self.mode {
            PromptMode::Progress {
                phase,
                processed,
                total,
            } => (*phase, *processed, *total),
            _ => return,
        };
        let result = match total {
            Some(t) => write!(self.progress_buf, "{phase} {processed}/{t} files"),
            None => write!(self.progress_buf, "{phase}... {processed} files"),
        };
        // write! で String への書き込みは失敗しない。万一に備え warn だけ残す
        if let Err(e) = result {
            tracing::warn!("failed to format progress text: {e}");
        }
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
            | PromptMode::Search { value, cursor, .. }
                if *cursor > 0 =>
            {
                *cursor -= 1;
                let byte_pos = char_to_byte_pos(value, *cursor);
                let next_byte_pos =
                    byte_pos + value[byte_pos..].chars().next().map_or(0, |c| c.len_utf8());
                value.replace_range(byte_pos..next_byte_pos, "");
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

    /// Progress 中はキー入力のほとんどを無視するが、Esc を受け取ったら Cancel Token を立て、
    /// 表示の phase を Cancelling に切り替える（worker は次の File-level Checkpoint で停止する）。
    fn handle_progress_event(&mut self, key: KeyEvent) -> Result<Action> {
        if key.code != KeyCode::Esc {
            return Ok(Action::None);
        }
        self.request_cancel();
        Ok(Action::None)
    }

    fn after_input_value_changed(&mut self) -> Action {
        self.mode.reset_candidates();
        if let PromptMode::Search { value, .. } = &self.mode {
            Action::SearchUpdate(value.clone())
        } else {
            Action::None
        }
    }

    /// Prompt を描画する。`keymap` はアイドル時（`PromptMode::None`）にのみ
    /// Commands 領域へ表示され、入力・進捗など他モードでは無視される。
    /// 本番では `render_main_view` がアクティブなコンポーネントの keymap を渡す。
    pub(crate) fn render_with_keymap(&self, frame: &mut Frame, area: Rect, keymap: &str) {
        let widget = match &self.mode {
            PromptMode::None => Paragraph::new(keymap)
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
            PromptMode::Progress { .. } => Paragraph::new(self.progress_buf.as_str())
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
            PromptMode::Progress { .. } => self.handle_progress_event(event),
            PromptMode::None => Ok(Action::None),
        }
    }

    // 本番描画は render_main_view が render_with_keymap を直接呼ぶ。
    // この trait 実装は Component 契約を満たすためのフォールバック（keymap 空）。
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.render_with_keymap(frame, area, "");
    }

    fn tick(&mut self) {
        // 溜まったメッセージを全て消費し、最新の進捗状態のみを反映する。
        // 各 try_recv は短いスコープに閉じて self.handle の借用を即座に解放し、
        // その後の self.apply_progress / self.clear_job (mutable self) と共存させる。
        loop {
            let result = match self.handle.as_ref() {
                Some(h) => h.rx.try_recv(),
                None => return,
            };
            match result {
                Ok(ProgressMessage::Update {
                    phase,
                    processed,
                    total,
                }) => self.apply_progress(phase, processed, total),
                Ok(ProgressMessage::Complete) => {
                    self.clear_job(PromptMode::None);
                    return;
                }
                Ok(ProgressMessage::Error(text)) => {
                    self.clear_job(PromptMode::Error { message: text });
                    return;
                }
                Err(mpsc::TryRecvError::Empty) => return,
                Err(mpsc::TryRecvError::Disconnected) => {
                    tracing::error!(
                        "async job progress channel disconnected before terminal message"
                    );
                    self.clear_job(PromptMode::Error {
                        message: "Progress channel disconnected unexpectedly".to_string(),
                    });
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
        PromptMode::Confirm { action, .. } => execute_confirm_action(ctx, action),
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

fn execute_confirm_action(ctx: &mut AppContext, action: ConfirmAction) -> Result<()> {
    match action {
        ConfirmAction::Delete { files } => {
            start_file_job(ctx, FileJob::Delete { files }, Phase::Scanning);
            Ok(())
        }
    }
}

/// Async Job を起動して PromptComponent に進捗ハンドルを渡す共通ヘルパ。
/// 既に別の Async Job が走っていれば `PromptMode::Error` を表示する (Filer Lock により
/// 通常は到達しないパスだが、不変条件破壊の早期検知のため tracing にも残す)。
fn start_file_job(ctx: &mut AppContext, job: FileJob, initial_phase: Phase) {
    if ctx.prompt.is_job_running() {
        tracing::warn!(
            "start_file_job called while another async job is running (Filer Lock invariant?)"
        );
        ctx.prompt
            .set_error("Another async job is already running".to_string());
        return;
    }
    let handle = spawn_async_job(move |cancel, on_progress| job.run(cancel, on_progress));
    ctx.prompt.start_async_job(handle, initial_phase);
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
        TextAction::Zip { dir, files } => {
            // 旧 fs::file::create_zip と同じ name 検証 (Unzip 経路と対称形)。
            // 絶対パスや `..` を含む name で `dir` の外に zip を書き出すのを防ぐ。
            anyhow::ensure!(
                !value.is_empty()
                    && std::path::Path::new(value)
                        .components()
                        .all(|c| matches!(c, std::path::Component::Normal(_))),
                "{value}: Invalid zip name"
            );
            start_file_job(
                ctx,
                FileJob::ZipCreate {
                    dir,
                    name: value.to_string(),
                    files,
                },
                Phase::Scanning,
            );
            Ok(())
        }
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

            start_file_job(
                ctx,
                FileJob::ZipExtract {
                    file,
                    dest: dest_path,
                },
                Phase::Extracting,
            );
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
            start_file_job(
                ctx,
                FileJob::Copy {
                    files,
                    dest: PathBuf::from(value),
                },
                Phase::Scanning,
            );
            Ok(())
        }
        FileAction::Move { files } => {
            start_file_job(
                ctx,
                FileJob::Move {
                    files,
                    dest: PathBuf::from(value),
                },
                Phase::Moving,
            );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::mpsc::{self, Sender};

    /// Async Job 実行中状態の PromptComponent を立ち上げるテストフィクスチャ。
    /// `worker_tx` から ProgressMessage を投入し、`tick()` でその反映を観察できる。
    struct ProgressFixture {
        prompt: PromptComponent,
        worker_tx: Sender<ProgressMessage>,
        cancel: Arc<AtomicBool>,
    }

    fn prompt_with_running_job(initial_phase: Phase) -> ProgressFixture {
        let (tx, rx) = mpsc::channel::<ProgressMessage>();
        let cancel = Arc::new(AtomicBool::new(false));
        let handle = AsyncJobHandle {
            rx,
            cancel: cancel.clone(),
        };
        let mut prompt = PromptComponent::new();
        prompt.start_async_job(handle, initial_phase);
        ProgressFixture {
            prompt,
            worker_tx: tx,
            cancel,
        }
    }

    /// PromptComponent を TestBackend に描画し、バッファ内容を1つの文字列にして返す。
    fn render_with_keymap_to_string(prompt: &mut PromptComponent, keymap: &str) -> String {
        let mut terminal = Terminal::new(TestBackend::new(80, 3)).expect("build test terminal");
        terminal
            .draw(|frame| {
                let area = frame.area();
                prompt.render_with_keymap(frame, area, keymap);
            })
            .expect("draw prompt");
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn idle_prompt_renders_the_supplied_keymap() {
        let mut prompt = PromptComponent::new();

        let text = render_with_keymap_to_string(&mut prompt, "q: Quit  ?: Help");

        assert!(text.contains("q: Quit  ?: Help"), "got: {text:?}");
    }

    #[test]
    fn active_prompt_renders_input_and_ignores_keymap() {
        let mut prompt = PromptComponent::new();
        prompt.mode = PromptMode::Text {
            title: "Grep".to_string(),
            value: "needle".to_string(),
            cursor: 0,
            action: Box::new(TextAction::Grep),
        };

        let text = render_with_keymap_to_string(&mut prompt, "q: Quit  ?: Help");

        assert!(
            text.contains("needle"),
            "input should render, got: {text:?}"
        );
        assert!(
            !text.contains("q: Quit"),
            "keymap must not leak into active mode, got: {text:?}"
        );
    }

    #[test]
    fn esc_during_progress_triggers_cancel_and_phase_switch() {
        let mut fx = prompt_with_running_job(Phase::Extracting);

        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let action = fx.prompt.handle_event(esc).expect("handle_event ok");

        // App は Action として何も伝搬を要求されない（Prompt 内部で吸収）
        assert!(matches!(action, Action::None));
        // cancel が立つ
        assert!(fx.cancel.load(Ordering::Relaxed));
        // mode の phase が Cancelling に切り替わる（processed/total は据え置き）
        match fx.prompt.mode {
            PromptMode::Progress { phase, .. } => assert_eq!(phase, Phase::Cancelling),
            other => panic!("expected Progress mode after Esc, got {other:?}"),
        }
    }

    #[test]
    fn tick_with_complete_clears_handle_and_returns_to_none_mode() {
        let mut fx = prompt_with_running_job(Phase::Extracting);
        fx.worker_tx.send(ProgressMessage::Complete).unwrap();

        fx.prompt.tick();

        assert!(matches!(fx.prompt.mode, PromptMode::None));
        assert!(!fx.prompt.is_job_running());
    }

    #[test]
    fn tick_with_error_switches_to_error_mode_with_message() {
        let mut fx = prompt_with_running_job(Phase::Extracting);
        fx.worker_tx
            .send(ProgressMessage::Error("disk full".into()))
            .unwrap();

        fx.prompt.tick();

        match &fx.prompt.mode {
            PromptMode::Error { message } => assert_eq!(message, "disk full"),
            other => panic!("expected Error mode, got {other:?}"),
        }
        assert!(!fx.prompt.is_job_running());
    }

    #[test]
    fn tick_reflects_received_update_into_progress_mode() {
        let mut fx = prompt_with_running_job(Phase::Extracting);
        fx.worker_tx
            .send(ProgressMessage::Update {
                phase: Phase::Extracting,
                processed: 7,
                total: Some(50),
            })
            .unwrap();

        fx.prompt.tick();

        match fx.prompt.mode {
            PromptMode::Progress {
                phase,
                processed,
                total,
            } => {
                assert_eq!(phase, Phase::Extracting);
                assert_eq!(processed, 7);
                assert_eq!(total, Some(50));
            }
            other => panic!("expected Progress mode, got {other:?}"),
        }
        assert!(fx.prompt.is_job_running());
    }
}
