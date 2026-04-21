use crate::fs::VFile;

#[derive(Debug)]
pub enum TextAction {
    Mkdir { dir: VFile },
    Rename { file: VFile },
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
    File {
        title: String,
        value: String,
        candidates: Vec<String>,
        candidate_index: Option<usize>,
    },
    Confirm {
        title: String,
        action: ConfirmAction,
    },
    Error {
        message: String,
    },
}

impl InputMode {
    pub fn is_active(&self) -> bool {
        !matches!(self, InputMode::None)
    }
}
