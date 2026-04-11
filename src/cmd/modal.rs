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
    let modal = std::mem::replace(&mut state.modal, ModalState::None);
    match modal {
        ModalState::DeleteConfirm { files } => {
            let mut error = None;
            for file in files {
                if let Err(e) = file.delete() {
                    error = Some(e);
                }
            }
            state.filer.refresh_files()?;
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
