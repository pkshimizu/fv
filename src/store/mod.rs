mod bookmark;
mod settings;
use anyhow::Result;

pub use bookmark::BookmarkStore;
pub use settings::{SettingsStore, StartupDirectory};

#[derive(Debug)]
pub struct RootStore {
    pub bookmark: BookmarkStore,
    pub settings: SettingsStore,
}

impl RootStore {
    pub fn new() -> Result<Self> {
        Ok(Self {
            bookmark: BookmarkStore::new()?,
            settings: SettingsStore::new()?,
        })
    }

    pub fn init(&mut self) -> Result<()> {
        self.bookmark.load()?;
        self.settings.load()?;
        Ok(())
    }
}
