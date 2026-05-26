use crate::fs::VFile;

/// 進捗データを表示用文字列に整形するトレイト。
/// `ProgressMessage::Update` で構造化データを送り、描画直前に 1 回だけ整形することで、
/// `try_recv` で吸い切られて捨てられるメッセージのフォーマットコストを避ける。
pub trait ProgressFormatter: Send {
    fn format(&self) -> String;
}

/// 非同期処理からの進捗メッセージ。
/// mpsc チャネル経由で PromptComponent に送信される。
pub enum ProgressMessage {
    /// 進捗状況の更新。整形は受信側の描画直前に遅延される。
    Update(Box<dyn ProgressFormatter + Send>),
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
        message: String,
    },
    Search {
        title: String,
        value: String,
        cursor: usize,
        original_index: Option<usize>,
    },
}

impl PromptMode {
    pub fn is_active(&self) -> bool {
        !matches!(self, PromptMode::None | PromptMode::Progress { .. })
    }

    /// テキスト入力モードの場合、カーソル位置と入力値を返す
    pub fn cursor_and_value(&self) -> Option<(usize, &str)> {
        match self {
            PromptMode::Text { cursor, value, .. }
            | PromptMode::File { cursor, value, .. }
            | PromptMode::Search { cursor, value, .. } => Some((*cursor, value)),
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
