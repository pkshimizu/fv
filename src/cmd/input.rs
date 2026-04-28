use crate::fs::VFile;
use crate::state::{
    AppState, ConfirmAction, FileAction, InputMode, SelectAction, SortKey, TextAction,
};
use anyhow::{Context, Result};
use std::path::Path;

pub fn input_char(state: &mut AppState, c: char) -> Result<()> {
    match &mut state.input {
        InputMode::Text { value, .. }
        | InputMode::File { value, .. }
        | InputMode::Search { value, .. } => {
            value.push(c);
        }
        _ => {}
    }
    after_input_value_changed(state);
    Ok(())
}

pub fn input_backspace(state: &mut AppState) -> Result<()> {
    match &mut state.input {
        InputMode::Text { value, .. }
        | InputMode::File { value, .. }
        | InputMode::Search { value, .. } => {
            value.pop();
        }
        _ => {}
    }
    after_input_value_changed(state);
    Ok(())
}

fn after_input_value_changed(state: &mut AppState) {
    state.input.reset_candidates();
    if let InputMode::Search { value, .. } = &state.input {
        state.filer.select_matching_file(value);
    }
}

pub fn input_select_left(state: &mut AppState) -> Result<()> {
    if let InputMode::Select {
        selected_index,
        options,
        ..
    } = &mut state.input
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
    if let InputMode::Select {
        selected_index,
        options,
        ..
    } = &mut state.input
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
    if let InputMode::File {
        value,
        candidates,
        candidate_index,
        ..
    } = &mut state.input
    {
        if candidates.is_empty() {
            *candidates = compute_path_candidates(value)?;
            if !candidates.is_empty() {
                *candidate_index = Some(0);
                *value = candidates[0].clone();
            }
        } else if let Some(index) = candidate_index {
            let next = (*index + 1) % candidates.len();
            *candidate_index = Some(next);
            *value = candidates[next].clone();
        }
    }
    Ok(())
}

pub fn input_ok(state: &mut AppState) -> Result<()> {
    let input = std::mem::replace(&mut state.input, InputMode::None);
    let skip_clear = matches!(
        input,
        InputMode::Select {
            action: SelectAction::Sort,
            ..
        }
    );
    match input {
        InputMode::Confirm { action, .. } => execute_confirm_action(state, action),
        InputMode::Text { action, value, .. } => execute_text_action(state, action, value.as_str()),
        InputMode::File { action, value, .. } => execute_file_action(state, action, value.as_str()),
        InputMode::Select {
            action,
            selected_index,
            ..
        } => execute_select_action(state, action, selected_index),
        InputMode::None | InputMode::Error { .. } | InputMode::Search { .. } => Ok(()),
    }?;
    if !skip_clear {
        state.filer.checked_paths.clear();
    }
    Ok(())
}

pub fn input_cancel(state: &mut AppState) -> Result<()> {
    state.input = InputMode::None;
    Ok(())
}

pub fn input_copy(state: &mut AppState) -> Result<()> {
    start_file_input(state, "Copy to", |files| FileAction::Copy { files })
}

pub fn input_delete(state: &mut AppState) -> Result<()> {
    let files = collect_action_targets(state);
    if !files.is_empty() {
        let title = action_title("Delete", &files);
        state.input = InputMode::Confirm {
            title,
            action: ConfirmAction::Delete { files },
        };
    }
    Ok(())
}

pub fn input_mkdir(state: &mut AppState) -> Result<()> {
    let dir = state.filer.current_dir.clone();
    if let Some(file_name) = dir.file_name() {
        let title = format!("Create directory in {file_name}");
        state.input = InputMode::Text {
            title,
            action: TextAction::Mkdir { dir },
            value: String::new(),
        };
    }
    Ok(())
}

pub fn input_move(state: &mut AppState) -> Result<()> {
    start_file_input(state, "Move to", |files| FileAction::Move { files })
}

pub fn input_rename(state: &mut AppState) -> Result<()> {
    let selected_file = state.filer.selected_file();
    if let Some(selected_file) = selected_file {
        if let Some(file_name) = selected_file.file_name() {
            let title = format!("Rename {file_name}");
            state.input = InputMode::Text {
                title,
                action: TextAction::Rename {
                    file: selected_file.clone(),
                },
                value: file_name.to_string(),
            };
        }
    }
    Ok(())
}

pub fn input_search(state: &mut AppState) -> Result<()> {
    state.input = InputMode::Search {
        title: "Search".to_string(),
        value: String::new(),
    };
    Ok(())
}

pub fn input_search_next(state: &mut AppState) -> Result<()> {
    if let InputMode::Search { value, .. } = &state.input {
        state.filer.select_next_matching_file(value);
    }
    Ok(())
}

pub fn input_search_prev(state: &mut AppState) -> Result<()> {
    if let InputMode::Search { value, .. } = &state.input {
        state.filer.select_prev_matching_file(value);
    }
    Ok(())
}

pub fn input_sort(state: &mut AppState) -> Result<()> {
    let options: Vec<String> = SortKey::ALL.iter().map(|k| k.label().to_string()).collect();
    let selected_index = state.filer.sort_key.index();
    state.input = InputMode::Select {
        title: "Sort by".to_string(),
        options,
        selected_index,
        action: SelectAction::Sort,
    };
    Ok(())
}

fn collect_action_targets(state: &AppState) -> Vec<VFile> {
    if state.filer.checked_paths.is_empty() {
        state.filer.selected_file().cloned().into_iter().collect()
    } else {
        state
            .filer
            .current_dir_files
            .iter()
            .filter(|file| state.filer.checked_paths.contains(file.absolute_path()))
            .cloned()
            .collect()
    }
}

fn start_file_input(
    state: &mut AppState,
    label: &str,
    make_action: impl FnOnce(Vec<VFile>) -> FileAction,
) -> Result<()> {
    let files = collect_action_targets(state);
    if !files.is_empty() {
        let title = action_title(label, &files);
        let init_value = if files.len() == 1 {
            files[0].absolute_path()
        } else {
            state.filer.current_dir.absolute_path()
        };
        state.input = InputMode::File {
            title,
            value: init_value.to_string(),
            candidates: Vec::new(),
            candidate_index: None,
            action: make_action(files),
        };
    }
    Ok(())
}

fn action_title(action_name: &str, files: &[VFile]) -> String {
    if files.len() == 1 {
        format!(
            "{} {}?",
            action_name,
            files[0].file_name().unwrap_or("(unknown)")
        )
    } else {
        format!("{} {} files?", action_name, files.len())
    }
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
