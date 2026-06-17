use crate::component::FilerComponent;
use ratatui_image::picker::Picker;

/// 1 つの作業 Context（CONTEXT.md 用語）。独立したカレントディレクトリ・カーソル・
/// Checked Paths 等を持つ Filer をまとめる単位。
///
/// 本構造体は複数 Context（#305）の土台。現状は `FilerComponent` のみをラップし、
/// 戻る/進む履歴の per-context 化（#317）や Context 操作（#305）で拡張していく。
pub struct FilerContext {
    filer: FilerComponent,
}

impl FilerContext {
    pub fn new(picker: Picker) -> Self {
        Self {
            filer: FilerComponent::new(picker),
        }
    }

    pub fn filer(&self) -> &FilerComponent {
        &self.filer
    }

    pub fn filer_mut(&mut self) -> &mut FilerComponent {
        &mut self.filer
    }
}
