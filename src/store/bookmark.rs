use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug)]
pub struct BookmarkStore {
    json_path: Option<PathBuf>,
    paths: HashSet<String>,
}

impl BookmarkStore {
    pub fn new() -> Self {
        BookmarkStore {
            json_path: None,
            paths: HashSet::new(),
        }
    }

    pub fn load(&mut self) -> Result<()> {
        let path = self.get_json_path()?;
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let paths: HashSet<String> =
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

    fn save(&mut self) -> Result<()> {
        let path = self.get_json_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create bookmarks config directory")?;
        }
        let content =
            serde_json::to_string_pretty(&self.paths).context("Failed to serialize bookmarks")?;
        std::fs::write(&path, content).context("Failed to write bookmarks file")?;
        Ok(())
    }

    pub fn add(&mut self, path: &str) -> Result<()> {
        self.paths.insert(path.to_string());
        self.save()?;
        Ok(())
    }

    pub fn remove(&mut self, path: &str) -> Result<()> {
        self.paths.remove(path);
        self.save()?;
        Ok(())
    }

    pub fn has(&self, path: &str) -> bool {
        self.paths.contains(path)
    }

    fn get_json_path(&mut self) -> Result<PathBuf> {
        if let Some(path) = &self.json_path {
            return Ok(path.clone());
        }
        let config_dir = dirs::config_dir().context("Failed to get config directory")?;
        let path = config_dir.join("fv").join("bookmarks.json");
        self.json_path = Some(path.clone());
        Ok(path)
    }
}
