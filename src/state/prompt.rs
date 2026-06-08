use crate::fs::VFile;

/// Async Job の現在フェーズ。
/// PromptComponent の進捗表示で `Copying 7/1234 files` のような表示文字列を組み立てる際に使う。
/// `Cancelling` は worker からは emit されず、Esc 受信時に PromptComponent が上書きする。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Scanning,
    Copying,
    Moving,
    Zipping,
    Extracting,
    Deleting,
    Cancelling,
}

impl std::fmt::Display for Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Phase::Scanning => "Scanning",
            Phase::Copying => "Copying",
            Phase::Moving => "Moving",
            Phase::Zipping => "Zipping",
            Phase::Extracting => "Extracting",
            Phase::Deleting => "Deleting",
            Phase::Cancelling => "Cancelling",
        })
    }
}

/// 非同期処理からの進捗メッセージ。
/// mpsc チャネル経由で PromptComponent に送信される。
#[derive(Debug)]
pub enum ProgressMessage {
    /// 進捗状況の更新。`total` が `None` の場合は Scan Phase など総量未確定状態を表す。
    Update {
        phase: Phase,
        processed: usize,
        total: Option<usize>,
    },
    /// 処理が正常に完了した
    Complete,
    /// 処理がエラーで終了した
    Error(String),
}

#[derive(Debug)]
pub enum TextAction {
    Mkdir { dir: VFile },
    Touch { dir: VFile },
    Rename { file: VFile },
    Zip { dir: VFile, files: Vec<VFile> },
    Unzip { file: VFile, dir: VFile },
    Grep,
    Execute { dir: VFile },
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
        cursor: usize,
        action: Box<TextAction>,
    },
    File {
        title: String,
        value: String,
        cursor: usize,
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
    Progress {
        phase: Phase,
        processed: usize,
        total: Option<usize>,
    },
    Search {
        title: String,
        value: String,
        cursor: usize,
        original_index: Option<usize>,
    },
    Filter {
        title: String,
        value: String,
        cursor: usize,
    },
}

impl PromptMode {
    pub fn is_active(&self) -> bool {
        !matches!(self, PromptMode::None)
    }

    /// テキスト入力モードの場合、カーソル位置と入力値を返す
    pub fn cursor_and_value(&self) -> Option<(usize, &str)> {
        match self {
            PromptMode::Text { cursor, value, .. }
            | PromptMode::File { cursor, value, .. }
            | PromptMode::Search { cursor, value, .. }
            | PromptMode::Filter { cursor, value, .. } => Some((*cursor, value)),
            _ => None,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_mode_is_active_so_filer_is_locked_during_async_job() {
        let progress = PromptMode::Progress {
            phase: Phase::Extracting,
            processed: 0,
            total: Some(10),
        };
        assert!(
            progress.is_active(),
            "Progress should be active so Filer ignores key events (Filer Lock)"
        );
        assert!(!PromptMode::None.is_active());
    }
}
