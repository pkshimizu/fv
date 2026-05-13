use crate::fs::VFile;

#[derive(Debug)]
pub enum TextAction {
    Mkdir { dir: VFile },
    Touch { dir: VFile },
    Rename { file: VFile },
    Grep,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

impl PromptMode {
    pub fn is_active(&self) -> bool {
        !matches!(self, PromptMode::None)
    }

    pub fn reset_candidates(&mut self) {
        if let PromptMode::File {
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
