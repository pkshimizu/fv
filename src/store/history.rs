use anyhow::{Context, Result};
use std::collections::VecDeque;
use std::path::PathBuf;

const MAX_HISTORY: usize = 1000;

#[derive(Debug)]
pub struct HistoryStore {
    json_path: PathBuf,
    entries: VecDeque<String>,
    cursor: usize,
}

impl HistoryStore {
    pub fn new() -> Result<Self> {
        let config_dir = dirs::config_dir().context("Failed to get config directory")?;
        let json_path = config_dir.join("fv").join("history.json");
        Ok(Self {
            json_path,
            entries: VecDeque::new(),
            cursor: 0,
        })
    }

    pub fn load(&mut self) -> Result<()> {
        let path = &self.json_path;
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let entries: Vec<String> =
                    serde_json::from_str(&content).context("Failed to parse history file")?;
                self.entries = VecDeque::from(entries);
                self.cursor = self.entries.len();
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                self.entries.clear();
                self.cursor = 0;
                Ok(())
            }
            Err(e) => Err(e).context("Failed to read history file"),
        }
    }

    fn save(&self) -> Result<()> {
        let json_path = &self.json_path;
        if let Some(parent) = json_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create history config directory")?;
        }
        let content =
            serde_json::to_string_pretty(&self.entries).context("Failed to serialize history")?;
        let tmp_path = json_path.with_extension("json.tmp");
        std::fs::write(&tmp_path, content).context("Failed to write history temp file")?;
        std::fs::rename(&tmp_path, json_path).context("Failed to save history file")?;
        Ok(())
    }

    pub fn add(&mut self, path: &str) -> Result<()> {
        // 直前と同じパスは追加しない
        if self.cursor > 0 && self.entries.get(self.cursor - 1).is_some_and(|e| e == path) {
            return Ok(());
        }
        // カーソルより後ろの履歴を削除（ブラウザの「進む」履歴をクリア）
        self.entries.truncate(self.cursor);
        self.entries.push_back(path.to_string());
        if self.entries.len() > MAX_HISTORY {
            self.entries.pop_front();
        }
        self.cursor = self.entries.len();
        self.save()
    }

    /// 一つ前の履歴に戻る。戻り先のパスを返す。
    pub fn back(&mut self) -> Option<&str> {
        if self.cursor > 1 {
            self.cursor -= 1;
            self.entries.get(self.cursor - 1).map(|s| s.as_str())
        } else {
            None
        }
    }

    /// 一つ先の履歴に進む。進み先のパスを返す。
    pub fn forward(&mut self) -> Option<&str> {
        if self.cursor < self.entries.len() {
            self.cursor += 1;
            self.entries.get(self.cursor - 1).map(|s| s.as_str())
        } else {
            None
        }
    }
}
