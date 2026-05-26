mod bookmark;
mod history;
mod settings;
use anyhow::Result;

pub use bookmark::BookmarkStore;
pub use history::HistoryStore;
pub use settings::{SettingsStore, StartupDirectory};

#[derive(Debug)]
pub struct RootStore {
    pub bookmark: BookmarkStore,
    pub settings: SettingsStore,
    pub history: HistoryStore,
}

impl RootStore {
    pub fn new() -> Result<Self> {
        Ok(Self {
            bookmark: BookmarkStore::new()?,
            settings: SettingsStore::new()?,
            history: HistoryStore::new()?,
        })
    }

    pub fn init(&mut self) -> Result<()> {
        self.bookmark.load()?;
        self.settings.load()?;
        self.history.load()?;
        Ok(())
    }
}
