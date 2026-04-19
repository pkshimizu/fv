use crate::fs::VFile;
use crate::state::{AppState, InputAction, InputMode};
use anyhow::Result;

pub fn input_char(state: &mut AppState, c: char) -> Result<()> {
    if let InputMode::Text { value, .. } = &mut state.input {
        value.push(c);
    }
    Ok(())
}

pub fn input_backspace(state: &mut AppState) -> Result<()> {
    if let InputMode::Text { value, .. } = &mut state.input {
        value.pop();
    }
    Ok(())
}

pub fn input_ok(state: &mut AppState) -> Result<()> {
    let input = std::mem::replace(&mut state.input, InputMode::None);
    match input {
        InputMode::Confirm { action, .. } => execute_confirm_action(state, action),
        InputMode::Text { action, value, .. } => execute_text_action(state, action, value.as_str()),
        InputMode::None => Ok(()),
    }
}

pub fn input_cancel(state: &mut AppState) -> Result<()> {
    state.input = InputMode::None;
    Ok(())
}

pub fn input_delete_confirm(state: &mut AppState) -> Result<()> {
    let files = collect_delete_targets(state);
    if !files.is_empty() {
        let title = delete_confirm_title(&files);
        state.input = InputMode::Confirm {
            title,
            action: InputAction::Delete { files },
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
            action: InputAction::Mkdir { dir },
            value: "".to_string(),
        };
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

fn execute_confirm_action(state: &mut AppState, action: InputAction) -> Result<()> {
    match action {
        InputAction::Delete { files } => execute_deletes(state, files),
        _ => Ok(()),
    }
}

fn execute_text_action(_: &mut AppState, action: InputAction, value: &str) -> Result<()> {
    match action {
        InputAction::Mkdir { dir } => execute_mkdir(dir, value),
        _ => Ok(()),
    }
}

fn execute_deletes(state: &mut AppState, files: Vec<VFile>) -> Result<()> {
    let mut error = None;
    for file in files {
        if let Err(e) = file.delete() {
            error.get_or_insert(e);
        }
    }
    if let Err(e) = state.filer.refresh_files() {
        error.get_or_insert(e);
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
