use crate::fs::VFile;

#[derive(Debug)]
pub enum TextAction {
    Mkdir { dir: VFile },
    Rename { file: VFile },
    Grep,
}

#[derive(Debug)]
pub enum ShellAction {
    Execute,
}

#[derive(Debug)]
pub enum ConfirmAction {
    Delete { files: Vec<VFile> },
}

#[derive(Debug)]
pub enum FileAction {
    Copy { files: Vec<VFile> },
    Move { files: Vec<VFile> },
    Jump,
}

#[derive(Debug)]
pub enum FileActionCandidateType {
    All,
    Directory,
}

#[derive(Debug)]
pub enum SelectAction {
    Sort,
}

#[derive(Debug, Default)]
pub enum PromptMode {
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
        candidate_type: FileActionCandidateType,
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
    Search {
        title: String,
        value: String,
        original_index: Option<usize>,
    },
    Shell {
        title: String,
        value: String,
        candidates: Vec<String>,
        candidate_index: Option<usize>,
        action: ShellAction,
    },
}

impl PromptMode {
    pub fn is_active(&self) -> bool {
        !matches!(self, PromptMode::None)
    }

    pub fn reset_candidates(&mut self) {
        match self {
            PromptMode::File {
                candidates,
                candidate_index,
                ..
            }
            | PromptMode::Shell {
                candidates,
                candidate_index,
                ..
            } => {
                candidates.clear();
                *candidate_index = None;
            }
            _ => {}
        }
    }
}
