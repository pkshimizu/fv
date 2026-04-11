use crate::state::{AppState, ModalState};
use anyhow::Result;
use std::fs;

pub fn open_delete_modal(state: &mut AppState) -> Result<()> {
    if state.filer.selected_file().is_some() {
        state.modal = ModalState::DeleteConfirm;
    }
    Ok(())
}

pub fn modal_confirm(state: &mut AppState) -> Result<()> {
    match &state.modal {
        ModalState::DeleteConfirm => {
            if let Some(file) = state.filer.selected_file() {
                let path = file.absolute_path().to_string();
                if file.is_dir()? {
                    fs::remove_dir_all(&path)?;
                } else {
                    fs::remove_file(&path)?;
                }
            }
            state.modal = ModalState::None;
            state.filer.refresh_files()?;
        }
        ModalState::None => {}
    }
    Ok(())
}

pub fn modal_cancel(state: &mut AppState) -> Result<()> {
    state.modal = ModalState::None;
    Ok(())
}
