use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct BookmarkStore {
    paths: Vec<String>,
}

impl BookmarkStore {
    pub fn new() -> Self {
        BookmarkStore { paths: vec![] }
    }

    pub fn load(&mut self) -> Result<()> {
        let path = self.json_path()?;
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let paths: Vec<String> =
                    serde_json::from_str(&content).context("Failed to parse bookmarks file")?;
                self.paths = paths;
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                self.paths.clear();
                Ok(())
            }
            Err(e) => Err(anyhow::Error::from(e).context("Failed to read bookmarks file")),
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = self.json_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create bookmarks config directory")?;
        }
        let content =
            serde_json::to_string_pretty(&self.paths).context("Failed to serialize bookmarks")?;
        std::fs::write(&path, content).context("Failed to write bookmarks file")?;
        Ok(())
    }

    pub fn add(&mut self, path: &str) {
        let target_path = path.to_string();
        if !self.paths.contains(&target_path) {
            self.paths.push(target_path);
        }
    }

    pub fn remove(&mut self, path: &str) {
        self.paths.retain(|p| p != &path.to_string());
    }

    pub fn has(&self, path: &str) -> bool {
        self.paths.contains(&path.to_string())
    }

    fn json_path(&self) -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("Failed to get config directory")?;
        Ok(config_dir.join("fv").join("bookmarks.json"))
    }
}
