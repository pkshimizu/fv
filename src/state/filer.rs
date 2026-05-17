use crate::fs::VFile;
use crate::state::table_cursor::TableCursor;
use anyhow::{Context, Result};
use ratatui::widgets::TableState;
use std::cmp::Ordering;
use std::collections::HashSet;

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
            .filter(|file| {
                self.show_dot_file || file.file_name().is_none_or(|name| !name.starts_with('.'))
            })
            .collect()
    }
}

#[derive(Debug)]
pub struct FilerState {
    pub current_dir: VFile,
    pub current_dir_files: Vec<VFile>,
    pub file_table_state: TableState,
    pub checked_paths: HashSet<String>,
    pub sort_key: SortKey,
    filter: FilerFilter,
    pending_select_name: Option<String>,
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
        self.load_current_dir(Some(VFile::new(current_dir_path)))?;

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

    pub fn change_to(&mut self, path: &str) -> Result<()> {
        self.load_current_dir(Some(VFile::new(path)))?;
        self.file_table_state.select(Some(0));
        Ok(())
    }

    pub fn change_dir_in_parent_dir(&mut self) -> Result<()> {
        let parent_dir = self.current_dir.parent_dir();
        if let Some(parent_dir) = parent_dir {
            self.change_to(parent_dir.absolute_path())?;
        }
        Ok(())
    }

    pub fn set_pending_select_name(&mut self, name: String) {
        self.pending_select_name = Some(name);
    }

    pub fn refresh_files(&mut self) -> Result<()> {
        let selected_name = self.pending_select_name.take().or_else(|| {
            self.selected_file()
                .and_then(|f| f.file_name().map(String::from))
        });

        self.load_current_dir(None)?;

        // 選択ファイル状態の更新
        if let Some(name) = selected_name {
            let new_index = self
                .current_dir_files
                .iter()
                .position(|f| f.file_name().unwrap_or_default() == name)
                .unwrap_or(0);
            self.file_table_state.select(Some(
                new_index.min(self.current_dir_files.len().saturating_sub(1)),
            ));
        } else {
            self.file_table_state.select(Some(0));
        }

        // チェック済みファイルの更新
        self.checked_paths.retain(|path| {
            self.current_dir_files
                .iter()
                .any(|file| file.absolute_path() == path.as_str())
        });

        Ok(())
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

    pub fn toggle_show_dot_file(&mut self) -> Result<()> {
        self.filter.show_dot_file = !self.filter.show_dot_file;
        self.refresh_files()
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

    fn load_current_dir(&mut self, current_dir: Option<VFile>) -> Result<()> {
        let mut files = if let Some(current_dir) = &current_dir {
            current_dir.list()?
        } else {
            self.current_dir.list()?
        };
        files = self.filter.apply(files);

        let sort_key = self.sort_key;
        files.sort_by(|a, b| {
            // ディレクトリ優先は常に維持
            match (a.is_dir(), b.is_dir()) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                (true, true) if !sort_key.is_apply_for_dirs() => a.file_name().cmp(&b.file_name()),
                _ => sort_key.compare(a, b),
            }
        });

        if let Some(current_dir) = current_dir {
            self.current_dir = current_dir;
        }

        self.current_dir_files = files;
        Ok(())
    }

    pub fn jump_to(&mut self, file_path: &str) -> Result<()> {
        let path = std::path::Path::new(file_path);
        let parent = path
            .parent()
            .and_then(|p| p.to_str())
            .context("Invalid path")?;
        self.change_to(parent)?;
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            self.set_pending_select_name(name.to_string());
            self.refresh_files()?;
        }
        Ok(())
    }
}
