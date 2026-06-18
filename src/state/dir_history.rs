use std::collections::VecDeque;

/// 戻る/進む履歴の保持上限。これを超えた分は古い方から捨てる。
/// 永続側の `store::history::HistoryStore` と同じ上限。
const MAX_HISTORY: usize = 1000;

/// 1 つの Context が持つ、戻る/進む用のディレクトリ履歴（インメモリ・非永続）。
///
/// 旧 `HistoryStore` のナビゲーション（`entries` + `cursor`）ロジックを Context 単位へ
/// 分離したもの。永続化（`history.json` / Startup の Last Directory）は
/// `store::history::HistoryStore` が引き続き担う。複数 Context（#305）では各 Context が
/// 独立した戻る/進むを持つ。
#[derive(Debug, Default)]
pub struct DirHistory {
    entries: VecDeque<String>,
    /// 次に記録する位置。`back`/`forward` で移動する。`entries.len()` のとき末尾（最新の次）。
    cursor: usize,
}

impl DirHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// 既存エントリ（永続履歴など）で初期化する。カーソルは末尾に置く。
    /// 起動時、最初の Context に永続履歴を引き継いで従来どおり戻れるようにするために使う。
    pub fn reset_with(&mut self, entries: Vec<String>) {
        self.entries = VecDeque::from(entries);
        self.cursor = self.entries.len();
    }

    /// 新しい訪問先を記録する。直前と同じパスは追加しない。カーソルより後ろ
    /// （「進む」履歴）は破棄する（ブラウザと同じ分岐挙動）。上限超過分は古い方から捨てる。
    pub fn push(&mut self, path: &str) {
        // 直前と同じパスは追加しない。
        if self.cursor > 0 && self.entries.get(self.cursor - 1).is_some_and(|e| e == path) {
            return;
        }
        // カーソルより後ろ（「進む」履歴）を破棄してから追加する。
        self.entries.truncate(self.cursor);
        self.entries.push_back(path.to_string());
        if self.entries.len() > MAX_HISTORY {
            self.entries.pop_front();
        }
        self.cursor = self.entries.len();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn back_and_forward_traverse_pushed_entries() {
        let mut h = DirHistory::new();
        h.push("/a");
        h.push("/b");
        h.push("/c");
        // 末尾にいるので back で一つずつ戻れる。
        assert_eq!(h.back(), Some("/b"));
        assert_eq!(h.back(), Some("/a"));
        // 先頭より前へは戻れない。
        assert_eq!(h.back(), None);
        // forward で戻した分だけ進める。
        assert_eq!(h.forward(), Some("/b"));
        assert_eq!(h.forward(), Some("/c"));
        // 末尾より先へは進めない。
        assert_eq!(h.forward(), None);
    }

    #[test]
    fn push_ignores_consecutive_duplicate() {
        let mut h = DirHistory::new();
        h.push("/a");
        h.push("/a");
        // 重複は記録されないので戻り先が無い。
        assert_eq!(h.back(), None);
    }

    #[test]
    fn push_after_back_truncates_forward_history() {
        let mut h = DirHistory::new();
        h.push("/a");
        h.push("/b");
        h.push("/c");
        assert_eq!(h.back(), Some("/b")); // /b に戻る
        h.push("/d"); // ここで /c（進む履歴）は破棄される
        assert_eq!(h.forward(), None); // 進む先は無い
        assert_eq!(h.back(), Some("/b")); // /d の一つ前は /b
    }

    #[test]
    fn cap_drops_oldest_entries() {
        let mut h = DirHistory::new();
        for i in 0..(MAX_HISTORY + 5) {
            h.push(&format!("/dir{i}"));
        }
        // 上限ぶんだけ戻れる（先頭まで戻ると None）。
        let mut count = 0;
        while h.back().is_some() {
            count += 1;
        }
        assert_eq!(count, MAX_HISTORY - 1);
    }

    #[test]
    fn reset_with_seeds_entries_and_puts_cursor_at_end() {
        let mut h = DirHistory::new();
        h.reset_with(vec!["/x".to_string(), "/y".to_string(), "/z".to_string()]);
        // 末尾（/z の次）から戻る。
        assert_eq!(h.back(), Some("/y"));
        assert_eq!(h.back(), Some("/x"));
        assert_eq!(h.back(), None);
    }

    #[test]
    fn empty_history_has_no_navigation() {
        let mut h = DirHistory::new();
        assert_eq!(h.back(), None);
        assert_eq!(h.forward(), None);
    }

    #[test]
    fn forward_right_after_seed_is_none() {
        // seed 直後はカーソルが末尾にあるので進む先は無い。
        let mut h = DirHistory::new();
        h.reset_with(vec!["/x".to_string(), "/y".to_string()]);
        assert_eq!(h.forward(), None);
    }

    #[test]
    fn push_after_seed_appends_as_continuation() {
        // 起動時 seed → ユーザー操作で push、という app.rs の経路を模す。
        let mut h = DirHistory::new();
        h.reset_with(vec!["/x".to_string(), "/y".to_string()]);
        h.push("/z");
        // /z の一つ前は seed 末尾の /y。
        assert_eq!(h.back(), Some("/y"));
        assert_eq!(h.back(), Some("/x"));
        assert_eq!(h.back(), None);
    }

    #[test]
    fn forward_works_after_back_hits_start() {
        let mut h = DirHistory::new();
        h.push("/a");
        h.push("/b");
        assert_eq!(h.back(), Some("/a")); // 先頭まで戻る
        assert_eq!(h.back(), None); // 下限に張り付く
        // 下限張り付き後も forward は正しく効く。
        assert_eq!(h.forward(), Some("/b"));
        assert_eq!(h.forward(), None);
    }
}
