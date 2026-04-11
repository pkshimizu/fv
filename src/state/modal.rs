use crate::fs::VFile;

#[derive(Debug, Clone, PartialEq)]
pub enum ModalState {
    None,
    DeleteConfirm { files: Vec<VFile> },
}

impl ModalState {
    pub fn is_active(&self) -> bool {
        !matches!(self, ModalState::None)
    }
}
