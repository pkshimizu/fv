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

#[derive(Debug)]
pub enum FileAction {
    Copy { files: Vec<VFile> },
    Move { files: Vec<VFile> },
}

#[derive(Debug)]
pub enum SelectAction {}

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
        action: FileAction,
    },
    Confirm {
        title: String,
        action: ConfirmAction,
    },
    Select {
        title: String,
        options: Vec<String>,
        selected_index: usize,
        action: SelectAction,
    },
    Error {
        message: String,
    },
}

impl InputMode {
    pub fn is_active(&self) -> bool {
        !matches!(self, InputMode::None)
    }

    pub fn reset_candidates(&mut self) {
        if let InputMode::File {
            candidates,
            candidate_index,
            ..
        } = self
        {
            candidates.clear();
            *candidate_index = None;
        }
    }
}
