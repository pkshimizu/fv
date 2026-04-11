use crate::fs::VFile;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum ModalState {
    # [default]
    None,
    DeleteConfirm { files: Vec<VFile> },
}

impl ModalState {
    pub fn is_active(&self) -> bool {
        !matches!(self, ModalState::None)
    }
}
