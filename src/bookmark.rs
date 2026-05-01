use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
struct BookmarkData {
    paths: Vec<String>,
}

fn bookmarks_file_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir().context("Failed to get config directory")?;
    Ok(config_dir.join("fv").join("bookmarks.json"))
}

pub fn load_bookmarks() -> Result<HashSet<String>> {
    let path = bookmarks_file_path()?;
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let data: BookmarkData = serde_json::from_str(&content)?;
            Ok(data.paths.into_iter().collect())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(HashSet::new()),
        Err(e) => Err(e.into()),
    }
}

pub fn save_bookmarks(bookmarks: &HashSet<String>) -> Result<()> {
    let path = bookmarks_file_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut paths: Vec<String> = bookmarks.iter().cloned().collect();
    paths.sort();
    let data = BookmarkData { paths };
    let content = serde_json::to_string_pretty(&data)?;
    std::fs::write(&path, content)?;
    Ok(())
}
