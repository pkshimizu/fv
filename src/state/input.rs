use crate::fs::VFile;

#[derive(Debug, Default)]
pub enum InputMode {
    #[default]
    None,
    Text {
        title: String,
        value: String,
    },
    Confirm {
        title: String,
    },
    DeleteConfirm {
        title: String,
        files: Vec<VFile>,
    },
}

impl InputMode {
    pub fn is_active(&self) -> bool {
        !matches!(self, InputMode::None)
    }
}
