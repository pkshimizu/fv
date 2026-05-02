use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Debug)]
pub struct BookmarkStore {
    json_path: PathBuf,
    paths: BTreeSet<String>,
}

impl BookmarkStore {
    pub fn new() -> Result<Self> {
        let config_dir = dirs::config_dir().context("Failed to get config directory")?;
        let json_path = config_dir.join("fv").join("bookmarks.json");
        Ok(BookmarkStore {
            json_path,
            paths: BTreeSet::new(),
        })
    }

    pub fn load(&mut self) -> Result<()> {
        let path = &self.json_path;
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let paths: BTreeSet<String> =
                    serde_json::from_str(&content).context("Failed to parse bookmarks file")?;
                self.paths = paths;
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                self.paths.clear();
                Ok(())
            }
            Err(e) => Err(e).context("Failed to read bookmarks file"),
        }
    }

    fn save(&self) -> Result<()> {
        let json_path = &self.json_path;
        if let Some(parent) = json_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .context("Failed to create bookmarks config directory")?;
            }
        }
        let content =
            serde_json::to_string_pretty(&self.paths).context("Failed to serialize bookmarks")?;
        std::fs::write(json_path, content).context("Failed to write bookmarks file")?;
        Ok(())
    }

    pub fn add(&mut self, path: &str) -> Result<()> {
        if self.paths.insert(path.to_string()) {
            self.save()?;
        }
        Ok(())
    }

    pub fn remove(&mut self, path: &str) -> Result<()> {
        if self.paths.remove(path) {
            self.save()?;
        }
        Ok(())
    }

    pub fn has(&self, path: &str) -> bool {
        self.paths.contains(path)
    }
}
