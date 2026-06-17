use crate::component::{Component, FilerComponent, PromptComponent};
use crate::os::disk_usage::DiskUsageReader;
use crate::os::system_info::SystemInfoReader;
use crate::state::{FilerContext, PasteBuffer, SidePanel};
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

    pub fn init(&mut self, startup_dir: Option<std::path::PathBuf>) -> Result<()> {
        self.active_filer_mut().init(startup_dir)
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn tick(&mut self) {
        // 全 Context を tick する（非アクティブ Context もディレクトリ読み込み等を進める）。
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
