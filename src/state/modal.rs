#[derive(Debug)]
pub enum ModalState {
    None,
    DeleteConfirm,
}

impl ModalState {
    pub fn is_active(&self) -> bool {
        !matches!(self, ModalState::None)
    }
}
