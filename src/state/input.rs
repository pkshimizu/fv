use crate::fs::VFile;

#[derive(Debug)]
pub enum InputAction {
    Delete { files: Vec<VFile> },
    Mkdir { dir: VFile },
}

#[derive(Debug, Default)]
pub enum InputMode {
    #[default]
    None,
    Text {
        title: String,
        value: String,
        action: InputAction,
    },
    Confirm {
        title: String,
        action: InputAction,
    },
}

impl InputMode {
    pub fn is_active(&self) -> bool {
        !matches!(self, InputMode::None)
    }
}
