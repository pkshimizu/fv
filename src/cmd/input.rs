use crate::fs::VFile;
use crate::state::{AppState, ConfirmAction, InputMode, TextAction};
use anyhow::Result;

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
            *candidates = compute_path_candidates(value);
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
        InputMode::File { .. } => Ok(()),
        InputMode::None | InputMode::Error { .. } => Ok(()),
    }
}

pub fn input_cancel(state: &mut AppState) -> Result<()> {
    state.input = InputMode::None;
    Ok(())
}

pub fn input_delete(state: &mut AppState) -> Result<()> {
    let files = collect_delete_targets(state);
    if !files.is_empty() {
        let title = delete_confirm_title(&files);
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
        let title = format!("Create directory in {}", file_name);
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
            let title = format!("Rename {}", file_name);
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

fn collect_delete_targets(state: &AppState) -> Vec<VFile> {
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

fn delete_confirm_title(files: &[VFile]) -> String {
    if files.len() == 1 {
        format!(
            "Delete {}?",
            files[0]
                .file_name()
                .unwrap_or_else(|| "(unknown)".to_string())
        )
    } else {
        format!("Delete {} files?", files.len())
    }
}

fn execute_confirm_action(_: &mut AppState, action: ConfirmAction) -> Result<()> {
    match action {
        ConfirmAction::Delete { files } => execute_deletes(files),
    }
}

fn execute_text_action(_: &mut AppState, action: TextAction, value: &str) -> Result<()> {
    match action {
        TextAction::Mkdir { dir } => execute_mkdir(dir, value),
        TextAction::Rename { file } => execute_rename(file, value),
    }
}

fn execute_deletes(files: Vec<VFile>) -> Result<()> {
    let mut error = None;
    for file in files {
        if let Err(e) = file.delete() {
            error.get_or_insert(e);
        }
    }
    if let Some(e) = error {
        return Err(e);
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

fn compute_path_candidates(input: &str) -> Vec<String> {
    use std::path::Path;

    let path = Path::new(input);
    let (dir, prefix) = if input.ends_with('/') {
        (path.to_path_buf(), String::new())
    } else {
        let dir = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| Path::new(".").to_path_buf());
        let prefix = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        (dir, prefix)
    };

    let Ok(entries) = std::fs::read_dir(&dir) else {
        return vec![];
    };

    let mut candidates: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.starts_with(&prefix)
        })
        .map(|e| {
            let full_path = dir.join(e.file_name());
            let mut s = full_path.to_string_lossy().to_string();
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                s.push('/');
            }
            s
        })
        .collect();

    candidates.sort();
    candidates
}
