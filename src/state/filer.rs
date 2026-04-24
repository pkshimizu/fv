use crate::fs::VFile;
use anyhow::{Context, Result};
use ratatui::widgets::TableState;
use std::cmp::Ordering;
use std::collections::HashSet;

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
    filter: FilerFilter,
}

impl FilerState {
    pub fn new() -> Self {
        Self {
            current_dir: VFile::new(""),
            current_dir_files: Vec::new(),
            file_table_state: TableState::default(),
            checked_paths: HashSet::new(),
            filter: FilerFilter::new(),
        }
    }

    pub fn init(&mut self) -> Result<()> {
        let home_dir = dirs::home_dir().context("Failed to get home directory")?;
        let current_dir_path = home_dir.to_str().context("Failed to get path string")?;
        self.load_current_dir(Some(VFile::new(current_dir_path)))?;

        self.file_table_state.select(Some(0));
        Ok(())
    }

    pub fn next(&mut self) {
        if self.current_dir_files.is_empty() {
            return;
        }
        if let Some(selected) = self.file_table_state.selected() {
            if selected < self.current_dir_files.len() - 1 {
                self.file_table_state.select(Some(selected + 1));
            }
        }
    }

    pub fn prev(&mut self) {
        if self.current_dir_files.is_empty() {
            return;
        }
        if let Some(selected) = self.file_table_state.selected() {
            if selected > 0 {
                self.file_table_state.select(Some(selected - 1));
            }
        }
    }

    pub fn first(&mut self) {
        if self.current_dir_files.is_empty() {
            return;
        }
        self.file_table_state.select(Some(0));
    }

    pub fn last(&mut self) {
        if self.current_dir_files.is_empty() {
            return;
        }
        self.file_table_state
            .select(Some(self.current_dir_files.len() - 1));
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

    pub fn refresh_files(&mut self) -> Result<()> {
        self.reload_current_dir()
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
        self.reload_current_dir()
    }

    fn reload_current_dir(&mut self) -> Result<()> {
        let selected_name = self
            .selected_file()
            .and_then(|f| f.file_name().map(String::from));

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

    fn load_current_dir(&mut self, current_dir: Option<VFile>) -> Result<()> {
        let mut files = if let Some(current_dir) = &current_dir {
            current_dir.list()?
        } else {
            self.current_dir.list()?
        };
        files = self.filter.apply(files);

        files.sort_by(|a, b| match (a.is_dir(), b.is_dir()) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        });

        if let Some(current_dir) = current_dir {
            self.current_dir = current_dir;
        }

        self.current_dir_files = files;
        Ok(())
    }
}
