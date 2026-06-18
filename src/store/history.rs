use anyhow::{Context, Result};
use std::collections::VecDeque;
use std::path::PathBuf;

const MAX_HISTORY: usize = 1000;

/// 訪問したディレクトリの永続ログ。セッションを跨いで保存され、Startup の Last Directory
/// （`last_entry`）の供給源になる。
///
/// 戻る/進む（`<`/`>`）のナビゲーションは Context ごとの `state::DirHistory` が担う。
/// 本ストアは永続化（追記・保存・読み込み）に専念し、カーソルは持たない。
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
            std::fs::create_dir_all(parent).context("Failed to create history config directory")?;
        }
        let content =
            serde_json::to_string_pretty(&self.entries).context("Failed to serialize history")?;
        let tmp_path = json_path.with_extension("json.tmp");
        std::fs::write(&tmp_path, content).context("Failed to write history temp file")?;
        std::fs::rename(&tmp_path, json_path).context("Failed to save history file")?;
        Ok(())
    }

    /// 訪問先を永続ログへ追記する。直前と同じパスは追加しない。上限超過分は古い方から捨てる。
    pub fn add(&mut self, path: &str) -> Result<()> {
        // 直前と同じパスは追加しない。
        if self.entries.back().is_some_and(|e| e == path) {
            return Ok(());
        }
        self.entries.push_back(path.to_string());
        if self.entries.len() > MAX_HISTORY {
            self.entries.pop_front();
        }
        self.save()
    }

    /// 履歴の最後のエントリを返す（Startup の Last Directory に使う）。
    pub fn last_entry(&self) -> Option<&str> {
        self.entries.back().map(|s| s.as_str())
    }

    /// 現在のエントリを順序どおりに複製して返す。起動時、最初の Context の
    /// `DirHistory` に永続履歴を引き継ぐために使う。
    pub fn entries_snapshot(&self) -> Vec<String> {
        self.entries.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// テスト用に一時ファイルパスを指すストアを作る（実設定ディレクトリを汚さない）。
    fn store(name: &str) -> HistoryStore {
        HistoryStore {
            json_path: std::env::temp_dir().join(format!("fv-history-test-{name}.json")),
            entries: VecDeque::new(),
        }
    }

    #[test]
    fn add_appends_and_last_entry_tracks_latest() {
        let mut s = store("append");
        s.add("/a").unwrap();
        s.add("/b").unwrap();
        assert_eq!(s.last_entry(), Some("/b"));
        assert_eq!(
            s.entries_snapshot(),
            vec!["/a".to_string(), "/b".to_string()]
        );
    }

    #[test]
    fn add_ignores_consecutive_duplicate() {
        let mut s = store("dedup");
        s.add("/a").unwrap();
        s.add("/a").unwrap();
        assert_eq!(s.entries_snapshot(), vec!["/a".to_string()]);
    }

    #[test]
    fn add_then_load_round_trips_through_disk() {
        let mut s = store("roundtrip");
        s.add("/p").unwrap();
        s.add("/q").unwrap();
        // 同じパスを指す別ストアで読み直すと内容が一致する。
        let mut reloaded = store("roundtrip");
        reloaded.load().unwrap();
        assert_eq!(
            reloaded.entries_snapshot(),
            vec!["/p".to_string(), "/q".to_string()]
        );
        assert_eq!(reloaded.last_entry(), Some("/q"));
    }

    #[test]
    fn missing_file_loads_as_empty() {
        let mut s = store("missing-file-unique");
        // 念のため事前に消しておく。
        let _ = std::fs::remove_file(&s.json_path);
        s.load().unwrap();
        assert!(s.entries_snapshot().is_empty());
        assert_eq!(s.last_entry(), None);
    }
}
