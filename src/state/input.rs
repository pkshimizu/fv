use crate::fs::VFile;

#[derive(Debug)]
pub enum TextAction {
    Mkdir { dir: VFile },
}

#[derive(Debug)]
pub enum ConfirmAction {
    Delete { files: Vec<VFile> },
}

#[derive(Debug, Default)]
pub enum InputMode {
    #[default]
    None,
    Text {
        title: String,
        value: String,
        action: TextAction,
    },
    Confirm {
        title: String,
        action: ConfirmAction,
    },
}

impl InputMode {
    pub fn is_active(&self) -> bool {
        !matches!(self, InputMode::None)
    }
}
