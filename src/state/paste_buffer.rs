/// Paste Buffer に取り込んだ操作モード。Copy は原本を残す、Cut は移動する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteMode {
    Copy,
    Cut,
}

/// Copy（Ctrl+C）/ Cut（Ctrl+X）で mark した対象のスナップショット。
/// Ctrl+V で現在表示中のディレクトリへ paste する。Yank（Operation Targets のパスを
/// システムクリップボードへ書き出す機能）とは別概念。
#[derive(Debug, Clone)]
pub struct PasteBuffer {
    /// 取り込み時点の対象（Operation Targets）の絶対パス列。
    pub paths: Vec<String>,
    pub mode: PasteMode,
}
