use crate::component::{Component, FilerComponent, PromptComponent};
use crate::os::system_info::SystemInfoReader;
use crate::state::SidePanel;
use anyhow::Result;
use ratatui_image::picker::Picker;

pub struct AppContext {
    pub running: bool,
    pub filer: FilerComponent,
    pub prompt: PromptComponent,
    pub side_panel: Option<SidePanel>,
    pub system_info: SystemInfoReader,
}

impl AppContext {
    pub fn new(picker: Picker) -> Self {
        Self {
            running: true,
            filer: FilerComponent::new(picker),
            prompt: PromptComponent::new(),
            side_panel: None,
            system_info: SystemInfoReader::new(),
        }
    }

    pub fn init(&mut self, startup_dir: Option<std::path::PathBuf>) -> Result<()> {
        self.filer.init(startup_dir)
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn tick(&mut self) {
        self.filer.tick();
        self.prompt.tick();
        self.system_info.tick();
        if let Some(panel) = &mut self.side_panel {
            panel.tick();
        }
    }
}
