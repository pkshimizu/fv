use crate::fs::VFile;
use crate::state::ProgressMessage;
use crate::state::table_cursor::TableCursor;
use anyhow::{Context, Result};
use ratatui::widgets::TableState;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::sync::mpsc;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SortKey {
    NameAsc,
    NameDesc,
    SizeAsc,
    SizeDesc,
    DateAsc,
    DateDesc,
}

impl SortKey {
    pub const ALL: [SortKey; 6] = [
        SortKey::NameAsc,
        SortKey::NameDesc,
        SortKey::SizeAsc,
        SortKey::SizeDesc,
        SortKey::DateAsc,
        SortKey::DateDesc,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            SortKey::NameAsc => "Name ↑",
            SortKey::NameDesc => "Name ↓",
            SortKey::SizeAsc => "Size ↑",
            SortKey::SizeDesc => "Size ↓",
            SortKey::DateAsc => "Date ↑",
            SortKey::DateDesc => "Date ↓",
        }
    }

    pub fn index(&self) -> usize {
        SortKey::ALL.iter().position(|k| k == self).unwrap_or(0)
    }

    fn is_apply_for_dirs(&self) -> bool {
        !matches!(self, SortKey::SizeAsc | SortKey::SizeDesc)
    }

    fn compare(&self, a: &VFile, b: &VFile) -> Ordering {
        match self {
            SortKey::NameAsc => a.file_name().cmp(&b.file_name()),
            SortKey::NameDesc => b.file_name().cmp(&a.file_name()),
            SortKey::SizeAsc | SortKey::SizeDesc => {
                let sa = a.metadata().map(|m| m.file_size()).unwrap_or(0);
                let sb = b.metadata().map(|m| m.file_size()).unwrap_or(0);
                let ord = sa.cmp(&sb);
                if matches!(self, SortKey::SizeDesc) {
                    ord.reverse()
                } else {
                    ord
                }
            }
            SortKey::DateAsc | SortKey::DateDesc => {
                let da = a.metadata().ok().and_then(|m| m.modified().ok());
                let db = b.metadata().ok().and_then(|m| m.modified().ok());
                let ord = da.cmp(&db);
                if matches!(self, SortKey::DateDesc) {
                    ord.reverse()
                } else {
                    ord
                }
            }
        }
    }
}

#[derive(Debug)]
struct FilerFilter {
    show_dot_file: bool,
}

impl FilerFilter {
    fn new() -> Self {
        Self {
            show_dot_file: false,
        }
    }

    fn apply(&self, files: Vec<VFile>) -> Vec<VFile> {
        files
            .into_iter()
            .filter(|file| self.should_include(file))
            .collect()
    }

    fn should_include(&self, file: &VFile) -> bool {
        self.show_dot_file || file.file_name().is_none_or(Self::is_visible_name)
    }

    /// ファイル名がフィルタ条件（ドットファイル非表示）で可視かどうかを判定する。
    /// should_include と start_async_load 内で共通利用される。
    fn is_visible_name(name: &str) -> bool {
        !name.starts_with('.')
    }
}

/// Operation Targets を解決した結果。集合だけでなく、Cursor File 由来か
/// Checked Paths 由来かという「由来」を保持する（CONTEXT.md 参照）。
/// いずれの variant も非空を不変条件とし、ターゲットが存在しない場合は
/// `operation_targets` が `None` を返す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationTargets {
    /// Checked Paths が空のため、Cursor File 単体を対象とする。
    Cursor(VFile),
    /// 多重選択。Checked Paths に一致する current_dir_files（常に非空）。
    Checked(Vec<VFile>),
}

impl OperationTargets {
    /// 実際に操作する VFile 列へ落とし込む。Copy / Move / Delete / Zip が利用する。
    pub fn into_files(self) -> Vec<VFile> {
        match self {
            OperationTargets::Cursor(file) => vec![file],
            OperationTargets::Checked(files) => files,
        }
    }
}

pub struct FilerState {
    pub current_dir: VFile,
    pub current_dir_files: Vec<VFile>,
    pub file_table_state: TableState,
    checked_paths: HashSet<String>,
    pub sort_key: SortKey,
    filter: FilerFilter,
    pending_select_name: Option<String>,
    dir_load_rx: Option<mpsc::Receiver<VFile>>,
    progress_rx: Option<mpsc::Receiver<ProgressMessage>>,
    load_error: Option<String>,
    prev_dir: Option<VFile>,
}

impl std::fmt::Debug for FilerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilerState")
            .field("current_dir", &self.current_dir)
            .field("current_dir_files", &self.current_dir_files.len())
            .field("sort_key", &self.sort_key)
            .field("pending_select_name", &self.pending_select_name)
            .field("is_loading", &self.dir_load_rx.is_some())
            .field("load_error", &self.load_error)
            .finish()
    }
}

impl FilerState {
    pub fn new() -> Self {
        Self {
            current_dir: VFile::new(""),
            current_dir_files: Vec::new(),
            file_table_state: TableState::default(),
            checked_paths: HashSet::new(),
            sort_key: SortKey::NameAsc,
            filter: FilerFilter::new(),
            pending_select_name: None,
            dir_load_rx: None,
            progress_rx: None,
            load_error: None,
            prev_dir: None,
        }
    }

    pub fn init(&mut self, startup_dir: Option<std::path::PathBuf>) -> Result<()> {
        let init_dir = if let Some(dir) = startup_dir {
            dir
        } else {
            std::env::current_dir()
                .ok()
                .or_else(dirs::home_dir)
                .context("Failed to get initial directory")?
        };
        let current_dir_path = init_dir.to_str().context("Failed to get path string")?;
        self.load_current_dir_sync(Some(VFile::new(current_dir_path)))?;

        self.file_table_state.select(Some(0));
        Ok(())
    }

    fn cursor(&mut self) -> TableCursor<'_> {
        TableCursor::new(&mut self.file_table_state, self.current_dir_files.len())
    }

    pub fn next(&mut self) {
        self.cursor().next();
    }

    pub fn prev(&mut self) {
        self.cursor().prev();
    }

    pub fn first(&mut self) {
        self.cursor().first();
    }

    pub fn last(&mut self) {
        self.cursor().last();
    }

    pub fn change_to(&mut self, path: &str) {
        self.start_async_load(Some(VFile::new(path)));
    }

    pub fn change_dir_in_parent_dir(&mut self) {
        if let Some(parent_dir) = self.current_dir.parent_dir() {
            self.start_async_load(Some(parent_dir));
        }
    }

    pub fn set_pending_select_name(&mut self, name: String) {
        self.pending_select_name = Some(name);
    }

    pub fn refresh_files(&mut self) {
        let selected_name = self.pending_select_name.take().or_else(|| {
            self.selected_file()
                .and_then(|f| f.file_name().map(String::from))
        });
        self.pending_select_name = selected_name;
        self.start_async_load(None);
    }

    pub fn selected_file(&self) -> Option<&VFile> {
        let selected_index = self.file_table_state.selected();
        selected_index.and_then(|i| self.current_dir_files.get(i))
    }

    pub fn is_checked(&self, file: &VFile) -> bool {
        self.is_checked_path(file.absolute_path())
    }

    fn is_checked_path(&self, path: &str) -> bool {
        self.checked_paths.contains(path)
    }

    pub fn toggle_checked_file(&mut self) {
        if let Some(selected_file) = self.selected_file() {
            let path = selected_file.absolute_path().to_string();
            if self.is_checked_path(&path) {
                self.checked_paths.remove(&path);
            } else {
                self.checked_paths.insert(path);
            }
        }
    }

    /// Checked Paths をすべて解除する。
    pub fn clear_checked_paths(&mut self) {
        self.checked_paths.clear();
    }

    /// Operation Targets を解決する。「Checked Paths が非空ならそれ、さもなくば
    /// Cursor File 単体」というルール（CONTEXT.md 参照）の唯一の実装。
    /// ターゲットが存在しない場合は `None`。
    pub fn operation_targets(&self) -> Option<OperationTargets> {
        if self.checked_paths.is_empty() {
            self.selected_file().cloned().map(OperationTargets::Cursor)
        } else {
            let files: Vec<VFile> = self
                .current_dir_files
                .iter()
                .filter(|file| self.is_checked_path(file.absolute_path()))
                .cloned()
                .collect();
            (!files.is_empty()).then_some(OperationTargets::Checked(files))
        }
    }

    pub fn show_dot_file(&self) -> bool {
        self.filter.show_dot_file
    }

    pub fn toggle_show_dot_file(&mut self) {
        self.filter.show_dot_file = !self.filter.show_dot_file;
        self.refresh_files();
    }

    pub fn select_matching_file(&mut self, query: &str) {
        if let Some(i) = self.find_matching_index(query, 0, true) {
            self.file_table_state.select(Some(i));
        }
    }

    pub fn select_next_matching_file(&mut self, query: &str) {
        let current = self.file_table_state.selected().unwrap_or(0);
        if let Some(i) = self.find_matching_index(query, current.wrapping_add(1), true) {
            self.file_table_state.select(Some(i));
        }
    }

    pub fn select_prev_matching_file(&mut self, query: &str) {
        let current = self.file_table_state.selected().unwrap_or(0);
        if let Some(i) = self.find_matching_index(query, current.wrapping_sub(1), false) {
            self.file_table_state.select(Some(i));
        }
    }

    fn find_matching_index(&self, query: &str, start: usize, forward: bool) -> Option<usize> {
        if query.is_empty() {
            return None;
        }
        let len = self.current_dir_files.len();
        if len == 0 {
            return None;
        }
        let start = start % len;
        let query_lower = query.to_lowercase();
        for step in 0..len {
            let i = if forward {
                (start + step) % len
            } else {
                (start + len - step) % len
            };
            if let Some(name) = self.current_dir_files[i].file_name()
                && name.to_lowercase().contains(&query_lower)
            {
                return Some(i);
            }
        }
        None
    }

    /// 起動時の同期ロード（init 専用）
    fn load_current_dir_sync(&mut self, current_dir: Option<VFile>) -> Result<()> {
        let mut files = if let Some(current_dir) = &current_dir {
            current_dir.list()?
        } else {
            self.current_dir.list()?
        };
        files = self.filter.apply(files);
        Self::sort_files(&mut files, self.sort_key);

        if let Some(current_dir) = current_dir {
            self.current_dir = current_dir;
        }

        self.current_dir_files = files;
        Ok(())
    }

    /// ディレクトリ走査を別スレッドで実行し、結果をmpscチャネルで受信する。
    fn start_async_load(&mut self, new_dir: Option<VFile>) {
        // 既存のロードをキャンセル
        self.dir_load_rx = None;
        self.progress_rx = None;
        self.load_error = None;

        self.prev_dir = Some(self.current_dir.clone());
        if let Some(new_dir) = new_dir {
            self.current_dir = new_dir;
        }

        self.current_dir_files.clear();
        self.file_table_state.select(None);

        let (file_tx, file_rx) = mpsc::channel::<VFile>();
        let (progress_tx, progress_rx) = mpsc::channel::<ProgressMessage>();

        self.dir_load_rx = Some(file_rx);
        self.progress_rx = Some(progress_rx);

        let dir_path = self.current_dir.absolute_path().to_string();
        let show_dot_file = self.filter.show_dot_file;

        std::thread::spawn(move || {
            let entries = match std::fs::read_dir(&dir_path) {
                Ok(entries) => entries,
                Err(e) => {
                    let _ = progress_tx.send(ProgressMessage::Error(format!("{e}")));
                    return;
                }
            };

            for entry in entries {
                let Ok(entry) = entry else { continue };

                // ドットファイルフィルタ（VFile構築前に判定して高速化）
                if !show_dot_file
                    && let Some(name) = entry.file_name().to_str()
                    && !FilerFilter::is_visible_name(name)
                {
                    continue;
                }

                let Some(path_str) = entry.path().to_str().map(String::from) else {
                    continue;
                };
                let vfile = VFile::new(path_str);
                if file_tx.send(vfile).is_err() {
                    return; // キャンセルされた
                }
            }
            let _ = progress_tx.send(ProgressMessage::Complete);
        });
    }

    /// tick ごとにチャネルからファイルを受信する
    pub fn receive_files(&mut self) {
        // 進捗チャネルからエラーを監視（Update/Complete は意図的に無視）
        if let Some(progress_rx) = &self.progress_rx {
            loop {
                match progress_rx.try_recv() {
                    Ok(ProgressMessage::Error(e)) => {
                        self.load_error = Some(e);
                        self.progress_rx = None;
                        self.dir_load_rx = None;
                        // エラー時は元のディレクトリに戻して同期リロード
                        if let Some(prev_dir) = self.prev_dir.take() {
                            if let Err(restore_err) = self.load_current_dir_sync(Some(prev_dir)) {
                                if let Some(err) = &mut self.load_error {
                                    err.push_str(&format!(" (restore failed: {restore_err})"));
                                }
                            } else {
                                self.file_table_state.select(Some(0));
                            }
                        }
                        return;
                    }
                    Ok(ProgressMessage::Complete) => {
                        self.progress_rx = None;
                        break;
                    }
                    Ok(_) => {} // Update は無視
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        self.progress_rx = None;
                        break;
                    }
                }
            }
        }

        let Some(rx) = &self.dir_load_rx else {
            return;
        };

        const MAX_RECV_PER_FRAME: usize = 200;
        let mut batch: Vec<VFile> = Vec::new();
        let mut disconnected = false;

        while batch.len() < MAX_RECV_PER_FRAME {
            match rx.try_recv() {
                Ok(file) => batch.push(file),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }

        // バッチをソートして既存リストとマージ（O(k log k + n)）
        if !batch.is_empty() {
            let sort_key = self.sort_key;
            Self::sort_files(&mut batch, sort_key);

            // 選択位置の補正: マージ前の選択ファイル名を記録
            let selected_name = self
                .file_table_state
                .selected()
                .and_then(|i| self.current_dir_files.get(i))
                .and_then(|f| f.file_name().map(String::from));

            // ソート済み同士のマージ
            let existing_files = std::mem::take(&mut self.current_dir_files);
            let mut merged = Vec::with_capacity(existing_files.len() + batch.len());
            let mut existing = existing_files.into_iter().peekable();
            let mut incoming = batch.into_iter().peekable();

            while existing.peek().is_some() || incoming.peek().is_some() {
                let take_existing = match (existing.peek(), incoming.peek()) {
                    (Some(a), Some(b)) => Self::compare_files(a, b, sort_key) != Ordering::Greater,
                    (Some(_), None) => true,
                    _ => false,
                };
                if take_existing {
                    merged.push(existing.next().unwrap());
                } else {
                    merged.push(incoming.next().unwrap());
                }
            }
            self.current_dir_files = merged;

            // 選択位置の復元
            if let Some(name) = selected_name {
                if let Some(idx) = self
                    .current_dir_files
                    .iter()
                    .position(|f| f.file_name().unwrap_or_default() == name)
                {
                    self.file_table_state.select(Some(idx));
                }
            } else if self.file_table_state.selected().is_none()
                && !self.current_dir_files.is_empty()
            {
                self.file_table_state.select(Some(0));
            }
        }

        if disconnected {
            self.dir_load_rx = None;
            self.progress_rx = None;
            self.finalize_loaded_files();
        }
    }

    /// 非同期ロードのエラーを取り出す
    pub fn take_error(&mut self) -> Option<String> {
        self.load_error.take()
    }

    /// 非同期ロード完了後の選択復元・チェック済みパスのクリーンアップ
    /// （ファイルは receive_files でソート済み挿入されるためソート不要）
    fn finalize_loaded_files(&mut self) {
        self.prev_dir = None;

        // 選択ファイル状態の復元
        if let Some(name) = self.pending_select_name.take() {
            let new_index = self
                .current_dir_files
                .iter()
                .position(|f| f.file_name().unwrap_or_default() == name)
                .unwrap_or(0);
            self.file_table_state.select(Some(
                new_index.min(self.current_dir_files.len().saturating_sub(1)),
            ));
        } else if !self.current_dir_files.is_empty() {
            self.file_table_state.select(Some(0));
        }

        // チェック済みファイルのクリーンアップ
        let file_paths: HashSet<&str> = self
            .current_dir_files
            .iter()
            .map(|f| f.absolute_path())
            .collect();
        self.checked_paths
            .retain(|path| file_paths.contains(path.as_str()));
    }

    pub fn is_loading(&self) -> bool {
        self.dir_load_rx.is_some()
    }

    fn compare_files(a: &VFile, b: &VFile, sort_key: SortKey) -> Ordering {
        // ディレクトリ優先は常に維持
        let ord = match (a.is_dir(), b.is_dir()) {
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            (true, true) if !sort_key.is_apply_for_dirs() => a.file_name().cmp(&b.file_name()),
            _ => sort_key.compare(a, b),
        };
        // 同値の場合はファイル名で安定化
        ord.then_with(|| a.file_name().cmp(&b.file_name()))
    }

    fn sort_files(files: &mut [VFile], sort_key: SortKey) {
        files.sort_by(|a, b| Self::compare_files(a, b, sort_key));
    }

    pub fn jump_to(&mut self, file_path: &str) -> Result<()> {
        let path = std::path::Path::new(file_path);
        let parent = path
            .parent()
            .and_then(|p| p.to_str())
            .context("Invalid path")?;
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            self.pending_select_name = Some(name.to_string());
        }
        self.change_to(parent);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 指定したパス列を current_dir_files に持ち、cursor を index に合わせた
    /// FilerState を組み立てる。VFile::new は存在しないパスでもパニックしない。
    fn state_with(files: &[&str], cursor: Option<usize>) -> FilerState {
        let mut state = FilerState::new();
        state.current_dir_files = files.iter().map(|p| VFile::new(*p)).collect();
        state.file_table_state.select(cursor);
        state
    }

    #[test]
    fn operation_targets_falls_through_to_cursor_file_when_unchecked() {
        let state = state_with(&["/a/foo.txt", "/a/bar.txt"], Some(1));

        let targets = state.operation_targets();

        assert_eq!(
            targets,
            Some(OperationTargets::Cursor(VFile::new("/a/bar.txt")))
        );
    }

    #[test]
    fn operation_targets_is_none_when_unchecked_and_no_cursor() {
        let state = state_with(&[], None);

        assert_eq!(state.operation_targets(), None);
    }

    #[test]
    fn operation_targets_uses_checked_paths_in_directory_order() {
        let mut state = state_with(&["/a/a.txt", "/a/b.txt", "/a/c.txt"], Some(0));
        // c を先に、a を後にチェックしても、結果は current_dir_files の順序を保つ。
        state.checked_paths.insert("/a/c.txt".to_string());
        state.checked_paths.insert("/a/a.txt".to_string());

        let targets = state.operation_targets();

        assert_eq!(
            targets,
            Some(OperationTargets::Checked(vec![
                VFile::new("/a/a.txt"),
                VFile::new("/a/c.txt"),
            ]))
        );
    }

    #[test]
    fn operation_targets_is_none_when_checked_paths_are_all_stale() {
        let mut state = state_with(&["/a/a.txt"], Some(0));
        // current_dir_files に存在しないパスだけがチェックされている状態。
        state.checked_paths.insert("/a/gone.txt".to_string());

        assert_eq!(state.operation_targets(), None);
    }

    #[test]
    fn clear_checked_paths_empties_the_set() {
        let mut state = state_with(&["/a/a.txt"], Some(0));
        state.checked_paths.insert("/a/a.txt".to_string());

        state.clear_checked_paths();

        // チェックが消えたので Cursor File へフォールスルーする。
        assert_eq!(
            state.operation_targets(),
            Some(OperationTargets::Cursor(VFile::new("/a/a.txt")))
        );
    }
}
