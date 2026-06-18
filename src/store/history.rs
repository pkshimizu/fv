use anyhow::{Context, Result};
use std::collections::VecDeque;
use std::path::PathBuf;

/// 永続ログの保持上限。これを超えた分は古い方から捨てる。
/// 起動時に seed で読み込む `state::DirHistory` の上限と揃えておくこと。
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
    use tempfile::TempDir;

    /// テスト用に、固有の一時ディレクトリ内の history.json を指すストアを作る。
    /// `TempDir` を返して呼び出し側で生存させ、Drop で自動削除させる
    /// （プロセス間で固定パスを共有せず、クリーンアップ漏れも防ぐ）。
    fn store() -> (HistoryStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = HistoryStore {
            json_path: dir.path().join("history.json"),
            entries: VecDeque::new(),
        };
        (store, dir)
    }

    #[test]
    fn add_appends_and_last_entry_tracks_latest() {
        let (mut s, _dir) = store();
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
        let (mut s, _dir) = store();
        s.add("/a").unwrap();
        s.add("/a").unwrap();
        assert_eq!(s.entries_snapshot(), vec!["/a".to_string()]);
    }

    #[test]
    fn add_drops_oldest_when_over_capacity() {
        let (mut s, _dir) = store();
        for i in 0..(MAX_HISTORY + 5) {
            s.add(&format!("/dir{i}")).unwrap();
        }
        let entries = s.entries_snapshot();
        assert_eq!(entries.len(), MAX_HISTORY);
        // 古い方（/dir0..=/dir4）が落ち、先頭は /dir5。
        assert_eq!(entries.first().map(String::as_str), Some("/dir5"));
        assert_eq!(s.last_entry(), Some("/dir1004"));
    }

    #[test]
    fn add_then_load_round_trips_through_disk() {
        let (mut s, dir) = store();
        s.add("/p").unwrap();
        s.add("/q").unwrap();
        // 同じパスを指す別ストアで読み直すと内容が一致する。
        let mut reloaded = HistoryStore {
            json_path: dir.path().join("history.json"),
            entries: VecDeque::new(),
        };
        reloaded.load().unwrap();
        assert_eq!(
            reloaded.entries_snapshot(),
            vec!["/p".to_string(), "/q".to_string()]
        );
        assert_eq!(reloaded.last_entry(), Some("/q"));
    }

    #[test]
    fn missing_file_loads_as_empty() {
        // 新規 TempDir 内に history.json はまだ無い。
        let (mut s, _dir) = store();
        s.load().unwrap();
        assert!(s.entries_snapshot().is_empty());
        assert_eq!(s.last_entry(), None);
    }
}
