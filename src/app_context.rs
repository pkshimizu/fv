use crate::component::{Component, FilerComponent, PromptComponent};
use crate::os::disk_usage::DiskUsageReader;
use crate::os::system_info::SystemInfoReader;
use crate::state::{DirHistory, FilerContext, PasteBuffer, SidePanel};
use anyhow::Result;
use ratatui_image::picker::Picker;

pub struct AppContext {
    pub running: bool,
    /// 保持している Context の一覧（必ず 1 つ以上）。複数 Context（#305）への土台。
    /// 現状は要素 1 で運用し、振る舞いは単一 Filer のときと変わらない。
    contexts: Vec<FilerContext>,
    /// アクティブな Context の `contexts` 上の位置（不変条件: `active < contexts.len()`）。
    active: usize,
    pub prompt: PromptComponent,
    pub side_panel: Option<SidePanel>,
    pub system_info: SystemInfoReader,
    pub disk_usage: DiskUsageReader,
    /// Copy/Cut で mark した対象（Ctrl+V で現在ディレクトリへ paste する）。
    pub paste_buffer: Option<PasteBuffer>,
}

impl AppContext {
    pub fn new(picker: Picker) -> Self {
        Self {
            running: true,
            contexts: vec![FilerContext::new(picker)],
            active: 0,
            prompt: PromptComponent::new(),
            side_panel: None,
            system_info: SystemInfoReader::new(),
            disk_usage: DiskUsageReader::new(),
            paste_buffer: None,
        }
    }

    /// アクティブ Context の Filer を参照する。
    pub fn active_filer(&self) -> &FilerComponent {
        self.contexts[self.active].filer()
    }

    /// アクティブ Context の Filer を可変参照する。
    pub fn active_filer_mut(&mut self) -> &mut FilerComponent {
        self.contexts[self.active].filer_mut()
    }

    /// アクティブ Context の戻る/進む履歴を可変参照する。
    pub fn active_history_mut(&mut self) -> &mut DirHistory {
        self.contexts[self.active].history_mut()
    }

    /// 保持している Context 数。
    pub fn context_count(&self) -> usize {
        self.contexts.len()
    }

    /// アクティブ Context の位置（タブバー表示用）。
    pub fn active_index(&self) -> usize {
        self.active
    }

    /// 各 Context のカレントディレクトリ（タブバー表示用、表示順）。
    pub fn context_dirs(&self) -> Vec<&str> {
        self.contexts
            .iter()
            .map(|c| c.filer().current_dir_path())
            .collect()
    }

    /// 現在ディレクトリを複製した新しい Context を、アクティブの直後に作りアクティブにする。
    pub fn new_context(&mut self) -> Result<()> {
        let dir = self.active_filer().current_dir_path().to_string();
        let context = self.contexts[self.active].duplicate_at(&dir)?;
        self.active += 1;
        self.contexts.insert(self.active, context);
        Ok(())
    }

    /// 次の Context へ切り替える（巡回）。
    pub fn next_context(&mut self) {
        self.active = (self.active + 1) % self.contexts.len();
    }

    /// 前の Context へ切り替える（巡回）。
    pub fn prev_context(&mut self) {
        self.active = (self.active + self.contexts.len() - 1) % self.contexts.len();
    }

    /// アクティブ Context をクローズする。最後の 1 つは閉じない（誤終了防止）。
    /// 閉じたら、同じ位置（末尾なら一つ前）の Context をアクティブにする。
    pub fn close_context(&mut self) {
        if self.contexts.len() <= 1 {
            return;
        }
        self.contexts.remove(self.active);
        if self.active >= self.contexts.len() {
            self.active = self.contexts.len() - 1;
        }
    }

    pub fn init(&mut self, startup_dir: Option<std::path::PathBuf>) -> Result<()> {
        self.active_filer_mut().init(startup_dir)
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn tick(&mut self) {
        // 全 Context を tick する。将来 Context が増えたとき、非アクティブ Context の
        // ディレクトリ読み込み等も進めるため（現状は要素 1 で旧 filer.tick() と等価）。
        for context in &mut self.contexts {
            context.filer_mut().tick();
        }
        self.prompt.tick();
        self.system_info.tick();
        self.disk_usage.tick();
        if let Some(panel) = &mut self.side_panel {
            panel.tick();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui_image::picker::Picker;
    use tempfile::TempDir;

    /// 既知の一時ディレクトリで初期化した AppContext を作る。
    fn context_in(dir: &TempDir) -> AppContext {
        let mut ctx = AppContext::new(Picker::halfblocks());
        ctx.init(Some(dir.path().to_path_buf())).unwrap();
        ctx
    }

    #[test]
    fn new_context_duplicates_dir_and_activates_it() {
        let dir = TempDir::new().unwrap();
        let mut ctx = context_in(&dir);
        let base = ctx.active_filer().current_dir_path().to_string();
        ctx.new_context().unwrap();
        assert_eq!(ctx.context_count(), 2);
        // 新規 Context はアクティブの直後に入りアクティブになる。
        assert_eq!(ctx.active_index(), 1);
        // 現在ディレクトリを複製している。
        assert_eq!(ctx.active_filer().current_dir_path(), base);
    }

    #[test]
    fn next_and_prev_cycle_through_contexts() {
        let dir = TempDir::new().unwrap();
        let mut ctx = context_in(&dir);
        ctx.new_context().unwrap(); // active=1, count=2
        ctx.next_context(); // 末尾の次は先頭へ巡回
        assert_eq!(ctx.active_index(), 0);
        ctx.next_context();
        assert_eq!(ctx.active_index(), 1);
        ctx.prev_context();
        assert_eq!(ctx.active_index(), 0);
        ctx.prev_context(); // 先頭の前は末尾へ巡回
        assert_eq!(ctx.active_index(), 1);
    }

    #[test]
    fn close_context_removes_and_clamps_active() {
        let dir = TempDir::new().unwrap();
        let mut ctx = context_in(&dir);
        ctx.new_context().unwrap(); // active=1, count=2
        ctx.close_context(); // 末尾を閉じ、active は一つ前へ
        assert_eq!(ctx.context_count(), 1);
        assert_eq!(ctx.active_index(), 0);
    }

    #[test]
    fn last_context_cannot_be_closed() {
        let dir = TempDir::new().unwrap();
        let mut ctx = context_in(&dir);
        ctx.close_context();
        assert_eq!(ctx.context_count(), 1);
    }
}
