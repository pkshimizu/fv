use crate::component::Component;
use crate::config::Config;
use crate::state::{FilerState, PromptMode, SidePanel};
use anyhow::Result;

#[derive(Debug, Eq, PartialEq)]
pub enum Area {
    Filer,
    Prompt,
    /// Component trait で処理するサイドパネル
    SideComponent,
}

pub struct AppState {
    pub config: Config,
    pub running: bool,
    pub launch_shell: bool,
    pub filer: FilerState,
    pub prompt: PromptMode,
    pub side_panel: Option<SidePanel>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            running: true,
            launch_shell: false,
            filer: FilerState::new(),
            prompt: PromptMode::None,
            side_panel: None,
        }
    }

    pub fn init(&mut self) -> Result<()> {
        self.filer.init()?;
        Ok(())
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn active_area(&self) -> Area {
        if self.prompt.is_active() {
            return Area::Prompt;
        }
        match &self.side_panel {
            Some(_) => Area::SideComponent,
            None => Area::Filer,
        }
    }

    pub fn is_active(&self, area: Area) -> bool {
        self.active_area() == area
    }

    pub fn tick(&mut self) {
        if let Some(panel) = &mut self.side_panel {
            panel.tick();
        }
    }
}
