use crate::fs::VFile;
use crate::state::{AppState, ModalState};
use anyhow::Result;

pub fn open_delete_modal(state: &mut AppState) -> Result<()> {
    if state.filer.checked_paths.is_empty() {
        if let Some(selected_file) = state.filer.selected_file() {
            state.modal = ModalState::DeleteConfirm {
                files: vec![selected_file.clone()],
            };
        }
    } else {
        let files = state
            .filer
            .checked_paths
            .iter()
            .map(|path| VFile::new(path))
            .collect();
        state.modal = ModalState::DeleteConfirm { files };
    }
    Ok(())
}

pub fn modal_confirm(state: &mut AppState) -> Result<()> {
    let modal = std::mem::replace(&mut state.modal, ModalState::None);
    match modal {
        ModalState::DeleteConfirm { files } => {
            let mut error = None;
            for file in files {
                if let Err(e) = file.delete() {
                    error.get_or_insert(e); // 最初のエラーのみ保持
                }
            }
            if let Err(e) = state.filer.refresh_files() {
                error.get_or_insert(e);
            }
            if let Some(e) = error {
                return Err(e);
            }
        }
        ModalState::None => {}
    }
    Ok(())
}

pub fn modal_cancel(state: &mut AppState) -> Result<()> {
    state.modal = ModalState::None;
    Ok(())
}
