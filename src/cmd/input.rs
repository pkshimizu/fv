use crate::fs::VFile;
use crate::state::{AppState, ConfirmAction, FileAction, InputMode, TextAction};
use anyhow::{Context, Result};
use std::path::Path;

pub fn input_char(state: &mut AppState, c: char) -> Result<()> {
    match &mut state.input {
        InputMode::Text { value, .. } => {
            value.push(c);
        }
        InputMode::File {
            value,
            candidates,
            candidate_index,
            ..
        } => {
            value.push(c);
            candidates.clear();
            *candidate_index = None;
        }
        _ => {}
    }
    Ok(())
}

pub fn input_backspace(state: &mut AppState) -> Result<()> {
    match &mut state.input {
        InputMode::Text { value, .. } => {
            value.pop();
        }
        InputMode::File {
            value,
            candidates,
            candidate_index,
            ..
        } => {
            value.pop();
            candidates.clear();
            *candidate_index = None;
        }
        _ => {}
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
    match input {
        InputMode::Confirm { action, .. } => execute_confirm_action(state, action),
        InputMode::Text { action, value, .. } => execute_text_action(state, action, value.as_str()),
        InputMode::File { action, value, .. } => execute_file_action(state, action, value.as_str()),
        InputMode::None | InputMode::Error { .. } => Ok(()),
    }
}

pub fn input_cancel(state: &mut AppState) -> Result<()> {
    state.input = InputMode::None;
    Ok(())
}

pub fn input_copy(state: &mut AppState) -> Result<()> {
    let files = collect_action_targets(state);
    if !files.is_empty() {
        let title = action_title("Copy to", &files);
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
            action: FileAction::Copy { files },
        };
    }
    Ok(())
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
                value: file_name,
            };
        }
    }
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

fn action_title(action_name: &str, files: &[VFile]) -> String {
    if files.len() == 1 {
        format!(
            "{} {}?",
            action_name,
            files[0]
                .file_name()
                .unwrap_or_else(|| "(unknown)".to_string())
        )
    } else {
        format!("{} {} files?", action_name, files.len())
    }
}

fn execute_confirm_action(state: &mut AppState, action: ConfirmAction) -> Result<()> {
    match action {
        ConfirmAction::Delete { files } => execute_deletes(files),
    }?;
    state.filer.checked_paths.clear();
    Ok(())
}

fn execute_text_action(state: &mut AppState, action: TextAction, value: &str) -> Result<()> {
    match action {
        TextAction::Mkdir { dir } => execute_mkdir(dir, value),
        TextAction::Rename { file } => execute_rename(file, value),
    }?;
    state.filer.checked_paths.clear();
    Ok(())
}

fn execute_file_action(state: &mut AppState, action: FileAction, value: &str) -> Result<()> {
    match action {
        FileAction::Copy { files } => execute_copy(files, value),
    }?;
    state.filer.checked_paths.clear();
    Ok(())
}

fn execute_copy(files: Vec<VFile>, value: &str) -> Result<()> {
    for file in &files {
        file.copy_to(value)?
    }
    Ok(())
}

fn execute_deletes(files: Vec<VFile>) -> Result<()> {
    for file in &files {
        file.delete()?
    }
    Ok(())
}

fn execute_mkdir(dir: VFile, value: &str) -> Result<()> {
    dir.create_dir(value)?;
    Ok(())
}

fn execute_rename(file: VFile, value: &str) -> Result<()> {
    file.rename(value)?;
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
