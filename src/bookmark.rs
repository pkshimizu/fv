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
            let data: BookmarkData =
                serde_json::from_str(&content).context("Failed to parse bookmarks file")?;
            Ok(data.paths.into_iter().collect())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(HashSet::new()),
        Err(e) => Err(anyhow::Error::from(e).context("Failed to read bookmarks file")),
    }
}

pub fn sorted_paths(bookmarks: &HashSet<String>) -> Vec<String> {
    let mut paths: Vec<String> = bookmarks.iter().cloned().collect();
    paths.sort();
    paths
}

pub fn save_bookmarks(bookmarks: &HashSet<String>) -> Result<()> {
    let path = bookmarks_file_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("Failed to create bookmarks config directory")?;
    }
    let data = BookmarkData {
        paths: sorted_paths(bookmarks),
    };
    let content = serde_json::to_string_pretty(&data).context("Failed to serialize bookmarks")?;
    std::fs::write(&path, content).context("Failed to write bookmarks file")?;
    Ok(())
}
