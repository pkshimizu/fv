use crate::config::Config;
use crate::state::FilerState;
use crate::state::{PathListState, PromptMode};
use anyhow::Result;
use std::sync::mpsc::TryRecvError;

#[derive(Debug, Eq, PartialEq)]
pub enum Area {
    Filer,
    Prompt,
    Bookmark,
    Grep,
}

#[derive(Debug)]
pub struct AppState {
    pub config: Config,
    pub running: bool,
    pub filer: FilerState,
    pub prompt: PromptMode,
    pub bookmark: Option<PathListState>,
    pub grep: Option<PathListState>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            running: true,
            filer: FilerState::new(),
            prompt: PromptMode::None,
            bookmark: None,
            grep: None,
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
        if self.grep.is_some() {
            return Area::Grep;
        }
        Area::Filer
    }

    pub fn is_active(&self, area: Area) -> bool {
        self.active_area() == area
    }

    pub fn receive_grep_results(&mut self) {
        let Some(grep) = &mut self.grep else {
            return;
        };

        let Some(rx) = &mut grep.rx else {
            return;
        };

        let mut count = 0;
        loop {
            match rx.try_recv() {
                Ok(path) => {
                    grep.paths.push(path);
                    count += 1;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    grep.rx = None;
                    break;
                }
            }
        }

        if count > 0 && grep.table_state.selected().is_none() {
            grep.table_state.select(Some(0));
        }
    }
}
