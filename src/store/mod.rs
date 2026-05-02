mod bookmark;
use anyhow::Result;

pub use bookmark::BookmarkStore;

#[derive(Debug)]
pub struct RootStore {
    pub bookmark: BookmarkStore,
}

impl RootStore {
    pub fn new() -> Self {
        Self {
            bookmark: BookmarkStore::new(),
        }
    }

    pub fn init(&mut self) -> Result<()> {
        self.bookmark.load()?;
        Ok(())
    }
}
