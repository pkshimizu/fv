use anyhow::{Context, Result};
use std::collections::VecDeque;
use std::path::PathBuf;

const MAX_HISTORY: usize = 1000;

#[derive(Debug)]
pub struct HistoryStore {
    json_path: PathBuf,
    entries: VecDeque<String>,
}

impl HistoryStore {
    pub fn new() -> Result<Self> {
        let config_dir = dirs::config_dir().context("Failed to get config directory")?;
        let json_path = config_dir.join("fv").join("history.json");
        Ok(Self {
            json_path,
            entries: VecDeque::new(),
        })
    }

    pub fn load(&mut self) -> Result<()> {
        let path = &self.json_path;
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let entries: Vec<String> =
                    serde_json::from_str(&content).context("Failed to parse history file")?;
                self.entries = VecDeque::from(entries);
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                self.entries.clear();
                Ok(())
            }
            Err(e) => Err(e).context("Failed to read history file"),
        }
    }

    fn save(&self) -> Result<()> {
        let json_path = &self.json_path;
        if let Some(parent) = json_path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create history config directory")?;
        }
        let entries: Vec<&String> = self.entries.iter().collect();
        let content =
            serde_json::to_string_pretty(&entries).context("Failed to serialize history")?;
        let tmp_path = json_path.with_extension("json.tmp");
        std::fs::write(&tmp_path, content).context("Failed to write history temp file")?;
        std::fs::rename(&tmp_path, json_path).context("Failed to save history file")?;
        Ok(())
    }

    pub fn add(&mut self, path: &str) -> Result<()> {
        // 直前と同じパスは追加しない
        if self.entries.back().is_some_and(|last| last == path) {
            return Ok(());
        }
        self.entries.push_back(path.to_string());
        if self.entries.len() > MAX_HISTORY {
            self.entries.pop_front();
        }
        self.save()
    }

    pub fn entries(&self) -> &VecDeque<String> {
        &self.entries
    }
}
