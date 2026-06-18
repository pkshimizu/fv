use crate::component::FilerComponent;
use crate::state::DirHistory;
use ratatui_image::picker::Picker;

/// 1 つの作業 Context（CONTEXT.md 用語）。独立したカレントディレクトリ・カーソル・
/// Checked Paths・戻る/進む履歴を持つ作業状態の単位。
///
/// 本構造体は複数 Context（#305）の土台。`FilerComponent`（カレントディレクトリ・カーソル・
/// Checked Paths 等）と `DirHistory`（戻る/進む履歴）をまとめる。Context 操作（#305）で
/// さらに拡張していく。
pub struct FilerContext {
    filer: FilerComponent,
    history: DirHistory,
}

impl FilerContext {
    pub fn new(picker: Picker) -> Self {
        Self {
            filer: FilerComponent::new(picker),
            history: DirHistory::new(),
        }
    }

    pub fn filer(&self) -> &FilerComponent {
        &self.filer
    }

    pub fn filer_mut(&mut self) -> &mut FilerComponent {
        &mut self.filer
    }

    pub fn history_mut(&mut self) -> &mut DirHistory {
        &mut self.history
    }
}
