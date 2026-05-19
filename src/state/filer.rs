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
        self.show_dot_file || file.file_name().is_none_or(|name| !name.starts_with('.'))
    }
}

pub struct FilerState {
    pub current_dir: VFile,
    pub current_dir_files: Vec<VFile>,
    pub file_table_state: TableState,
    pub checked_paths: HashSet<String>,
    pub sort_key: SortKey,
    filter: FilerFilter,
    pending_select_name: Option<String>,
    dir_load_rx: Option<mpsc::Receiver<VFile>>,
    progress_rx: Option<mpsc::Receiver<ProgressMessage>>,
    load_error: Option<String>,
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

    fn cursor(&mut self) -> TableCursor {
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
            if let Some(name) = self.current_dir_files[i].file_name() {
                if name.to_lowercase().contains(&query_lower) {
                    return Some(i);
                }
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

            let mut count = 0u64;
            for entry in entries {
                let Ok(entry) = entry else { continue };

                // ドットファイルフィルタ（VFile構築前に判定して高速化）
                if !show_dot_file {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.starts_with('.') {
                            continue;
                        }
                    }
                }

                let Some(path_str) = entry.path().to_str().map(String::from) else {
                    continue;
                };
                let vfile = VFile::new(path_str);
                if file_tx.send(vfile).is_err() {
                    return; // キャンセルされた
                }
                count += 1;
                if count % 100 == 0 {
                    let _ = progress_tx
                        .send(ProgressMessage::Update(format!("Loading... {count} files")));
                }
            }
            let _ = progress_tx.send(ProgressMessage::Complete);
        });
    }

    /// tick ごとにチャネルからファイルを受信する
    pub fn receive_files(&mut self) {
        // 進捗チャネルからエラーを監視
        if let Some(progress_rx) = &self.progress_rx {
            while let Ok(msg) = progress_rx.try_recv() {
                if let ProgressMessage::Error(e) = msg {
                    self.load_error = Some(e);
                    self.progress_rx = None;
                    self.dir_load_rx = None;
                    return;
                }
            }
        }

        let Some(rx) = &self.dir_load_rx else {
            return;
        };

        const MAX_RECV_PER_FRAME: usize = 200;
        let mut count = 0;
        let mut disconnected = false;

        while count < MAX_RECV_PER_FRAME {
            match rx.try_recv() {
                Ok(file) => {
                    self.current_dir_files.push(file);
                    count += 1;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }

        if disconnected {
            self.dir_load_rx = None;
            self.progress_rx = None;
            self.finalize_loaded_files();
        } else if count > 0
            && self.file_table_state.selected().is_none()
            && !self.current_dir_files.is_empty()
        {
            self.file_table_state.select(Some(0));
        }
    }

    /// 非同期ロードのエラーを取り出す
    pub fn take_error(&mut self) -> Option<String> {
        self.load_error.take()
    }

    /// 非同期ロード完了後のソート・選択復元・チェック済みパスのクリーンアップ
    fn finalize_loaded_files(&mut self) {
        Self::sort_files(&mut self.current_dir_files, self.sort_key);

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
        self.checked_paths.retain(|path| {
            self.current_dir_files
                .iter()
                .any(|file| file.absolute_path() == path.as_str())
        });
    }

    pub fn is_loading(&self) -> bool {
        self.dir_load_rx.is_some()
    }

    fn sort_files(files: &mut [VFile], sort_key: SortKey) {
        files.sort_by(|a, b| {
            // ディレクトリ優先は常に維持
            match (a.is_dir(), b.is_dir()) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                (true, true) if !sort_key.is_apply_for_dirs() => a.file_name().cmp(&b.file_name()),
                _ => sort_key.compare(a, b),
            }
        });
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
