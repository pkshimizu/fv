use crate::state::{AppState, InputMode};
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
        InputMode::DeleteConfirm { files, .. } => {
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
        }
        InputMode::None | InputMode::Text { .. } => {}
    }
    Ok(())
}

pub fn input_cancel(state: &mut AppState) -> Result<()> {
    state.input = InputMode::None;
    Ok(())
}

pub fn open_delete_confirm(state: &mut AppState) -> Result<()> {
    if state.filer.checked_paths.is_empty() {
        if let Some(selected_file) = state.filer.selected_file() {
            let title = format!(
                "Delete {}?",
                selected_file
                    .file_name()
                    .unwrap_or_else(|| "(unknown)".to_string())
            );
            state.input = InputMode::DeleteConfirm {
                title,
                files: vec![selected_file.clone()],
            };
        }
    } else {
        let files: Vec<_> = state
            .filer
            .current_dir_files
            .iter()
            .filter(|file| state.filer.checked_paths.contains(file.absolute_path()))
            .cloned()
            .collect();
        let title = if files.len() == 1 {
            format!(
                "Delete {}?",
                files[0]
                    .file_name()
                    .unwrap_or_else(|| "(unknown)".to_string())
            )
        } else {
            format!("Delete {} files?", files.len())
        };
        state.input = InputMode::DeleteConfirm { title, files };
    }
    Ok(())
}
