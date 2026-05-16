use crate::component::{Component, FilerComponent, PromptComponent};
use crate::config::Config;
use crate::state::SidePanel;
use anyhow::Result;

pub struct AppState {
    pub config: Config,
    pub running: bool,
    pub filer: FilerComponent,
    pub prompt: PromptComponent,
    pub side_panel: Option<SidePanel>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            running: true,
            filer: FilerComponent::new(),
            prompt: PromptComponent::new(),
            side_panel: None,
        }
    }

    pub fn init(&mut self) -> Result<()> {
        self.filer.init()
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn tick(&mut self) {
        if let Some(panel) = &mut self.side_panel {
            panel.tick();
        }
    }
}
