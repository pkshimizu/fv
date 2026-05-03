use crate::config::Config;
use crate::state::FilerState;
use crate::state::PromptMode;
use crate::state::bookmark::BookmarkState;
use anyhow::Result;

#[derive(Debug, Eq, PartialEq)]
pub enum Area {
    Filer,
    Prompt,
    Bookmark,
}

#[derive(Debug)]
pub struct AppState {
    pub config: Config,
    pub running: bool,
    pub filer: FilerState,
    pub prompt: PromptMode,
    pub bookmark: Option<BookmarkState>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            running: true,
            filer: FilerState::new(),
            prompt: PromptMode::None,
            bookmark: None,
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
        if self.bookmark.is_some() {
            return Area::Bookmark;
        }
        Area::Filer
    }

    pub fn is_active(&self, area: Area) -> bool {
        self.active_area() == area
    }
}
