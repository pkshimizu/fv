use crate::state::{AppState, ModalState};
use anyhow::Result;

pub fn open_delete_modal(state: &mut AppState) -> Result<()> {
    if let Some(selected_file) = state.filer.selected_file() {
        state.modal = ModalState::DeleteConfirm {
            files: vec![selected_file.clone()],
        };
    }
    Ok(())
}

pub fn modal_confirm(state: &mut AppState) -> Result<()> {
    match &state.modal {
        ModalState::DeleteConfirm { files } => {
            for file in files {
                file.delete()?;
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
