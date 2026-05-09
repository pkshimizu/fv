use crate::fs::VFile;
use crate::state::{
    AppState, ConfirmAction, FileAction, PathListState, PromptMode, SelectAction, ShellAction,
    SidePanel, SortKey, TextAction,
};
use anyhow::{Context, Result};
use std::io::BufRead;
use std::path::Path;

pub fn input_char(state: &mut AppState, c: char) -> Result<()> {
    match &mut state.prompt {
        PromptMode::Text { value, .. }
        | PromptMode::File { value, .. }
        | PromptMode::Search { value, .. }
        | PromptMode::Shell { value, .. } => {
            value.push(c);
        }
        _ => {}
    }
    after_input_value_changed(state);
    Ok(())
}

pub fn input_backspace(state: &mut AppState) -> Result<()> {
    match &mut state.prompt {
        PromptMode::Text { value, .. }
        | PromptMode::File { value, .. }
        | PromptMode::Search { value, .. }
        | PromptMode::Shell { value, .. } => {
            value.pop();
        }
        _ => {}
    }
    after_input_value_changed(state);
    Ok(())
}

fn after_input_value_changed(state: &mut AppState) {
    state.prompt.reset_candidates();
    if let PromptMode::Search { value, .. } = &state.prompt {
        state.filer.select_matching_file(value);
    }
}

pub fn input_select_left(state: &mut AppState) -> Result<()> {
    if let PromptMode::Select {
        selected_index,
        options,
        ..
    } = &mut state.prompt
    {
        if *selected_index > 0 {
            *selected_index -= 1;
        } else {
            *selected_index = options.len().saturating_sub(1);
        }
    }
    Ok(())
}

pub fn input_select_right(state: &mut AppState) -> Result<()> {
    if let PromptMode::Select {
        selected_index,
        options,
        ..
    } = &mut state.prompt
    {
        if *selected_index + 1 < options.len() {
            *selected_index += 1;
        } else {
            *selected_index = 0;
        }
    }
    Ok(())
}

pub fn input_tab(state: &mut AppState) -> Result<()> {
    match &mut state.prompt {
        PromptMode::File {
            value,
            candidates,
            candidate_index,
            ..
        } => {
            cycle_candidates(
                value,
                candidates,
                candidate_index,
                CycleDirection::Forward,
                Some(compute_path_candidates),
            )?;
        }
        PromptMode::Shell {
            value,
            candidates,
            candidate_index,
            ..
        } => {
            cycle_candidates(
                value,
                candidates,
                candidate_index,
                CycleDirection::Forward,
                Some(compute_shell_candidates),
            )?;
        }
        _ => {}
    }
    Ok(())
}

pub fn input_back_tab(state: &mut AppState) -> Result<()> {
    match &mut state.prompt {
        PromptMode::File {
            value,
            candidates,
            candidate_index,
            ..
        }
        | PromptMode::Shell {
            value,
            candidates,
            candidate_index,
            ..
        } => {
            cycle_candidates(
                value,
                candidates,
                candidate_index,
                CycleDirection::Backward,
                None,
            )?;
        }
        _ => {}
    }
    Ok(())
}

type ComputeCandidates = fn(&str) -> Result<Vec<String>>;

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

pub fn input_ok(state: &mut AppState) -> Result<()> {
    // Search モードでは Enter でカーソル位置を維持したまま検索を終了する
    if matches!(state.prompt, PromptMode::Search { .. }) {
        state.prompt = PromptMode::None;
        return Ok(());
    }
    let input = std::mem::replace(&mut state.prompt, PromptMode::None);
    let skip_clear = matches!(
        input,
        PromptMode::Select {
            action: SelectAction::Sort,
            ..
        }
    );
    match input {
        PromptMode::Confirm { action, .. } => execute_confirm_action(state, action),
        PromptMode::Text { action, value, .. } => {
            execute_text_action(state, action, value.as_str())
        }
        PromptMode::File { action, value, .. } => {
            execute_file_action(state, action, value.as_str())
        }
        PromptMode::Shell { action, value, .. } => {
            execute_shell_action(state, action, value.as_str())
        }
        PromptMode::Select {
            action,
            selected_index,
            ..
        } => execute_select_action(state, action, selected_index),
        PromptMode::None | PromptMode::Error { .. } | PromptMode::Search { .. } => Ok(()),
    }?;
    if !skip_clear {
        state.filer.checked_paths.clear();
    }
    Ok(())
}

pub fn input_cancel(state: &mut AppState) -> Result<()> {
    if let PromptMode::Search { original_index, .. } = &state.prompt {
        state.filer.file_table_state.select(*original_index);
    }
    state.prompt = PromptMode::None;
    Ok(())
}

pub fn input_search_next(state: &mut AppState) -> Result<()> {
    if let PromptMode::Search { value, .. } = &state.prompt {
        state.filer.select_next_matching_file(value);
    }
    Ok(())
}

pub fn input_search_prev(state: &mut AppState) -> Result<()> {
    if let PromptMode::Search { value, .. } = &state.prompt {
        state.filer.select_prev_matching_file(value);
    }
    Ok(())
}

fn execute_confirm_action(_: &mut AppState, action: ConfirmAction) -> Result<()> {
    match action {
        ConfirmAction::Delete { files } => execute_delete(files),
    }
}

fn execute_text_action(state: &mut AppState, action: TextAction, value: &str) -> Result<()> {
    match action {
        TextAction::Mkdir { dir } => execute_mkdir(dir, value),
        TextAction::Rename { file } => execute_rename(state, file, value),
        TextAction::Grep => execute_grep(state, value),
    }
}

fn execute_file_action(_: &mut AppState, action: FileAction, value: &str) -> Result<()> {
    match action {
        FileAction::Copy { files } => execute_copy(files, value),
        FileAction::Move { files } => execute_move(files, value),
    }
}

fn execute_select_action(
    state: &mut AppState,
    action: SelectAction,
    selected_index: usize,
) -> Result<()> {
    match action {
        SelectAction::Sort => {
            if let Some(&sort_key) = SortKey::ALL.get(selected_index) {
                state.filer.sort_key = sort_key;
                state.filer.refresh_files()?;
            }
            Ok(())
        }
    }
}

fn execute_copy(files: Vec<VFile>, value: &str) -> Result<()> {
    for file in &files {
        file.copy_to(value)?
    }
    Ok(())
}

fn execute_move(files: Vec<VFile>, value: &str) -> Result<()> {
    for file in &files {
        file.move_to(value)?
    }
    Ok(())
}

fn execute_delete(files: Vec<VFile>) -> Result<()> {
    for file in &files {
        file.delete()?
    }
    Ok(())
}

fn execute_mkdir(dir: VFile, value: &str) -> Result<()> {
    dir.create_dir(value)?;
    Ok(())
}

fn execute_rename(state: &mut AppState, file: VFile, value: &str) -> Result<()> {
    file.rename(value)?;
    state.filer.set_pending_select_name(value.to_string());
    Ok(())
}

fn execute_grep(state: &mut AppState, value: &str) -> Result<()> {
    if value.is_empty() {
        return Ok(());
    }

    let dir_path = state.filer.current_dir.absolute_path().to_string();

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

    // grep実行時は既存のサイドパネルを置き換える（ユーザーが明示的に検索を実行した操作のため）
    state.side_panel = Some(SidePanel::Grep(PathListState::new(Vec::new(), Some(rx))));
    Ok(())
}

fn execute_shell_action(_state: &mut AppState, action: ShellAction, _command: &str) -> Result<()> {
    match action {
        ShellAction::Execute => {
            // TODO: シェルコマンドの実行は次のタスクで対応
            // 注意: sh -c 経由ではなく Command::new(program).args(args) で直接実行すること
            Ok(())
        }
    }
}

const MAX_SHELL_CANDIDATES: usize = 1000;

fn compute_shell_candidates(prefix: &str) -> Result<Vec<String>> {
    if prefix.is_empty() {
        return Ok(Vec::new());
    }

    // PATH未設定時は候補なしとして扱う
    let path_var = std::env::var("PATH").unwrap_or_default();
    let mut candidates = Vec::new();

    'outer: for dir in path_var.split(':') {
        let dir_path = Path::new(dir);
        // 存在しないディレクトリや権限不足はスキップ
        let Ok(entries) = std::fs::read_dir(dir_path) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(prefix) && is_executable(&entry) {
                candidates.push(name);
                if candidates.len() >= MAX_SHELL_CANDIDATES {
                    break 'outer;
                }
            }
        }
    }

    candidates.sort();
    candidates.dedup();
    Ok(candidates)
}

fn is_executable(entry: &std::fs::DirEntry) -> bool {
    use std::os::unix::fs::PermissionsExt;
    entry
        .metadata()
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

fn compute_path_candidates(input: &str) -> Result<Vec<String>> {
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
