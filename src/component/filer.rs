use crate::component::{
    Action, AttributeComponent, AudioPlayerComponent, Component, FileInfoComponent, HelpComponent,
    ImagePreviewComponent, PreviewComponent, TreeComponent,
};
use crate::fs::VFile;
use crate::fs::file_info::{is_audio_file, is_image_file};
use crate::state::{
    ConfirmAction, FileAction, FileActionCandidateType, FilerState, OperationTargets, PromptMode,
    SelectAction, SidePanel, SortKey, TextAction,
};
use crate::store::RootStore;
use crate::ui::widgets::{BorderStyle, Spinner, build_bordered_block};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Cell, Row, Table};
use ratatui_image::picker::Picker;

use crate::fs::VFileTime;
use std::path::Path;

const DOTFILE_STYLE: Style = Style::new().fg(Color::Blue);
const DIR_STYLE: Style = Style::new().fg(Color::Green);
const CHECKED_SYMBOL: &str = "*";
const BOOKMARK_SYMBOL: &str = "B";

pub struct FilerComponent {
    state: FilerState,
    active: bool,
    picker: Picker,
    spinner: Spinner,
}

impl FilerComponent {
    pub fn new(picker: Picker) -> Self {
        Self {
            state: FilerState::new(),
            active: true,
            picker,
            spinner: Spinner::new(),
        }
    }

    pub fn init(&mut self, startup_dir: Option<std::path::PathBuf>) -> Result<()> {
        self.state.init(startup_dir)
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub fn current_dir_path(&self) -> &str {
        self.state.current_dir.absolute_path()
    }

    pub fn jump_to(&mut self, path: &str) -> Result<()> {
        self.state.jump_to(path)
    }

    pub fn change_to(&mut self, path: &str) {
        self.state.change_to(path);
    }

    pub fn refresh_files(&mut self) {
        self.state.refresh_files();
    }

    pub fn is_loading(&self) -> bool {
        self.state.is_loading()
    }

    pub fn take_error(&mut self) -> Option<String> {
        self.state.take_error()
    }

    pub fn select_matching_file(&mut self, value: &str) {
        self.state.select_matching_file(value);
    }

    pub fn select_next_matching_file(&mut self, value: &str) {
        self.state.select_next_matching_file(value);
    }

    pub fn select_prev_matching_file(&mut self, value: &str) {
        self.state.select_prev_matching_file(value);
    }

    pub fn clear_checked_paths(&mut self) {
        self.state.clear_checked_paths();
    }

    pub fn set_pending_select_name(&mut self, name: String) {
        self.state.set_pending_select_name(name);
    }

    pub fn set_sort_key(&mut self, key: SortKey) {
        self.state.sort_key = key;
    }

    pub fn select_file_table(&mut self, index: Option<usize>) {
        self.state.file_table_state.select(index);
    }

    fn action_title(action_name: &str, files: &[VFile]) -> String {
        match files {
            [file] => format!(
                "{} {}?",
                action_name,
                file.file_name().unwrap_or("(unknown)")
            ),
            _ => format!("{} {} files?", action_name, files.len()),
        }
    }

    fn prompt_file_input(
        &self,
        label: &str,
        candidate_type: FileActionCandidateType,
        make_action: impl FnOnce(Vec<VFile>) -> FileAction,
    ) -> Action {
        let Some(targets) = self.state.operation_targets() else {
            return Action::None;
        };
        let files = targets.into_files();
        let title = Self::action_title(label, &files);
        let init_value = match files.as_slice() {
            [file] => file.absolute_path(),
            _ => self.state.current_dir.absolute_path(),
        };
        let value = init_value.to_string();
        let cursor = value.chars().count();
        Action::SetPromptMode(Box::new(PromptMode::File {
            title,
            value,
            cursor,
            candidate_type,
            candidates: Vec::new(),
            candidate_index: None,
            action: make_action(files),
        }))
    }

    fn prompt_copy(&self) -> Action {
        self.prompt_file_input("Copy to", FileActionCandidateType::All, |files| {
            FileAction::Copy { files }
        })
    }

    fn prompt_move(&self) -> Action {
        self.prompt_file_input("Move to", FileActionCandidateType::All, |files| {
            FileAction::Move { files }
        })
    }

    fn prompt_delete(&self) -> Action {
        let Some(targets) = self.state.operation_targets() else {
            return Action::None;
        };
        let files = targets.into_files();
        let title = Self::action_title("Delete", &files);
        Action::SetPromptMode(Box::new(PromptMode::Confirm {
            title,
            action: ConfirmAction::Delete { files },
        }))
    }

    fn prompt_mkdir(&self) -> Action {
        let dir = self.state.current_dir.clone();
        if let Some(file_name) = dir.file_name() {
            let title = format!("Create directory in {file_name}");
            Action::SetPromptMode(Box::new(PromptMode::Text {
                title,
                value: String::new(),
                cursor: 0,
                action: Box::new(TextAction::Mkdir { dir }),
            }))
        } else {
            Action::None
        }
    }

    fn prompt_touch(&self) -> Action {
        let dir = self.state.current_dir.clone();
        if let Some(file_name) = dir.file_name() {
            let title = format!("Create file in {file_name}");
            Action::SetPromptMode(Box::new(PromptMode::Text {
                title,
                value: String::new(),
                cursor: 0,
                action: Box::new(TextAction::Touch { dir }),
            }))
        } else {
            Action::None
        }
    }

    fn prompt_zip(&self) -> Action {
        let Some(targets) = self.state.operation_targets() else {
            return Action::None;
        };
        let dir = self.state.current_dir.clone();
        // デフォルト名は Operation Targets の由来で決まる（CONTEXT.md 参照）。
        // Cursor File 由来ならその stem から、Checked Paths 由来なら汎用名。
        let default_name = match &targets {
            OperationTargets::Cursor(file) => {
                let stem = file.file_stem().unwrap_or("archive");
                format!("{stem}.zip")
            }
            OperationTargets::Checked(_) => "files.zip".to_string(),
        };
        let files = targets.into_files();
        let default_path = Path::new(dir.absolute_path()).join(&default_name);
        let value = default_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or(default_name);
        let title = Self::action_title("Zip", &files);
        let cursor = value.chars().count();
        Action::SetPromptMode(Box::new(PromptMode::Text {
            title,
            value,
            cursor,
            action: Box::new(TextAction::Zip { dir, files }),
        }))
    }

    fn prompt_unzip(&self) -> Action {
        let Some(file) = self.state.selected_file() else {
            return Action::None;
        };
        let Some(file_name) = file.file_name() else {
            return Action::None;
        };
        // 圧縮ファイルかどうかを拡張子で判定
        let path = Path::new(file_name);
        let is_archive = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"));
        if !is_archive {
            return Action::SetPromptMode(Box::new(PromptMode::Error {
                message: format!("{file_name} is not a zip file"),
            }));
        }
        let dir = self.state.current_dir.clone();
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "extracted".to_string());
        let title = format!("Unzip {file_name} to");
        let cursor = stem.chars().count();
        Action::SetPromptMode(Box::new(PromptMode::Text {
            title,
            value: stem,
            cursor,
            action: Box::new(TextAction::Unzip {
                file: file.clone(),
                dir,
            }),
        }))
    }

    fn prompt_rename(&self) -> Action {
        let Some(selected_file) = self.state.selected_file() else {
            return Action::None;
        };
        let Some(file_name) = selected_file.file_name() else {
            return Action::None;
        };
        let title = format!("Rename {file_name}");
        let value = file_name.to_string();
        let cursor = value.chars().count();
        Action::SetPromptMode(Box::new(PromptMode::Text {
            title,
            value,
            cursor,
            action: Box::new(TextAction::Rename {
                file: selected_file.clone(),
            }),
        }))
    }

    fn prompt_sort(&self) -> Action {
        let options: Vec<String> = SortKey::ALL.iter().map(|k| k.label().to_string()).collect();
        let selected_index = self.state.sort_key.index();
        Action::SetPromptMode(Box::new(PromptMode::Select {
            title: "Sort by".to_string(),
            options,
            selected_index,
            action: SelectAction::Sort,
        }))
    }

    fn prompt_search(&self) -> Action {
        let original_index = self.state.file_table_state.selected();
        Action::SetPromptMode(Box::new(PromptMode::Search {
            title: "Search".to_string(),
            value: String::new(),
            cursor: 0,
            original_index,
        }))
    }

    fn prompt_grep(&self) -> Action {
        Action::SetPromptMode(Box::new(PromptMode::Text {
            title: "Grep".to_string(),
            value: String::new(),
            cursor: 0,
            action: Box::new(TextAction::Grep),
        }))
    }

    fn prompt_jump(&self) -> Action {
        let value = self.state.current_dir.absolute_path().to_string();
        let cursor = value.chars().count();
        Action::SetPromptMode(Box::new(PromptMode::File {
            title: "Jump".to_string(),
            value,
            cursor,
            candidate_type: FileActionCandidateType::Directory,
            candidates: Vec::new(),
            candidate_index: None,
            action: FileAction::Jump,
        }))
    }

    fn prompt_execute(&self) -> Action {
        let Some(file) = self.state.selected_file() else {
            return Action::None;
        };
        let dir = self.state.current_dir.clone();
        let raw_path = file.absolute_path();
        let path = format!("'{}'", raw_path.replace('\'', "'\\''"));
        let cursor = path.chars().count();
        Action::SetPromptMode(Box::new(PromptMode::Text {
            title: "Execute".to_string(),
            value: path,
            cursor,
            action: Box::new(TextAction::Execute { dir }),
        }))
    }

    fn enter_file(&mut self) -> Result<Action> {
        let Some(file) = self.state.selected_file() else {
            return Ok(Action::None);
        };
        if file.is_dir() {
            let path = file.absolute_path().to_string();
            self.state.change_to(&path);
            Ok(Action::None)
        } else {
            Ok(Action::OpenFile(file.absolute_path().to_string()))
        }
    }

    fn show_attribute(&self) -> Result<Action> {
        let Some(file) = self.state.selected_file() else {
            return Ok(Action::None);
        };
        Ok(Action::ShowSidePanel(Box::new(SidePanel::Attribute(
            AttributeComponent::new(file)?,
        ))))
    }

    fn show_file_info(&self) -> Result<Action> {
        let Some(file) = self.state.selected_file() else {
            return Ok(Action::None);
        };
        Ok(Action::ShowSidePanel(Box::new(SidePanel::FileInfo(
            FileInfoComponent::new(file)?,
        ))))
    }

    fn show_tree(&self) -> Action {
        let current_path = if let Some(file) = self.state.selected_file() {
            file.absolute_path().to_string()
        } else {
            self.state.current_dir.absolute_path().to_string()
        };
        let show_dot_file = self.state.show_dot_file();
        Action::ShowSidePanel(Box::new(SidePanel::Tree(TreeComponent::new(
            &current_path,
            show_dot_file,
        ))))
    }

    fn show_preview(&self) -> Result<Action> {
        let Some(file) = self.state.selected_file() else {
            return Ok(Action::None);
        };
        if file.is_dir() {
            return Ok(Action::None);
        }
        let path = file.absolute_path();
        let file_name = file.file_name().unwrap_or("(unknown)");

        let panel = if is_audio_file(path) {
            AudioPlayerComponent::new(path, file_name).map(SidePanel::AudioPlayer)
        } else if is_image_file(path) {
            ImagePreviewComponent::new(path, file_name, &self.picker).map(SidePanel::ImagePreview)
        } else {
            PreviewComponent::new(path, file_name).map(SidePanel::Preview)
        };
        match panel {
            Ok(p) => Ok(Action::ShowSidePanel(Box::new(p))),
            Err(e) => Ok(Action::SetPromptMode(Box::new(PromptMode::Error {
                message: format!("Failed to preview: {e}"),
            }))),
        }
    }

    fn build_file_table(&self, block: Block<'static>, store: &RootStore) -> Table<'static> {
        let files = &self.state.current_dir_files;
        let rows: Vec<Row> = files
            .iter()
            .filter_map(|file| {
                let metadata = file.metadata().ok()?;
                let checked = if self.state.is_checked(file) {
                    CHECKED_SYMBOL
                } else {
                    " "
                };
                let file_name = file.file_name().unwrap_or_default();
                let is_dotfile = file_name.starts_with('.');
                let is_dir = metadata.is_dir();
                let is_bookmarked = store.bookmark.has(file.absolute_path());
                let row = Row::new(vec![
                    Cell::from(checked),
                    Cell::from(file_name.to_string()),
                    Cell::from(Text::from(if is_bookmarked {
                        BOOKMARK_SYMBOL
                    } else {
                        " "
                    })),
                    Cell::from(metadata.permissions().to_rwx_string()),
                    Cell::from(
                        Text::from(if is_dir {
                            "<dir>".to_string()
                        } else {
                            metadata.compact_size()
                        })
                        .alignment(Alignment::Right),
                    ),
                    Cell::from(format_time(metadata.modified())),
                ]);
                let row = if is_dir {
                    row.style(DIR_STYLE)
                } else if is_dotfile {
                    row.style(DOTFILE_STYLE)
                } else {
                    row
                };
                Some(row)
            })
            .collect();
        Table::new(
            rows,
            [
                Constraint::Max(1),
                Constraint::Fill(1),
                Constraint::Max(1),
                Constraint::Max(9),
                Constraint::Max(10),
                Constraint::Max(19),
            ],
        )
        .block(block)
        .highlight_symbol("> ")
        .row_highlight_style(Style::default().add_modifier(Modifier::UNDERLINED))
    }
}

impl Component for FilerComponent {
    fn keymap(&self) -> &'static str {
        "q: Quit  ?: Help"
    }

    fn tick(&mut self) {
        self.spinner.advance();
        self.state.receive_files();
    }

    // キーバインドを変更した場合は help.rs の KEY_BINDINGS も更新すること。
    fn handle_event(&mut self, event: KeyEvent) -> Result<Action> {
        match event.code {
            KeyCode::Up => {
                self.state.prev();
                Ok(Action::None)
            }
            KeyCode::Down => {
                self.state.next();
                Ok(Action::None)
            }
            KeyCode::Left => {
                self.state.first();
                Ok(Action::None)
            }
            KeyCode::Right => {
                self.state.last();
                Ok(Action::None)
            }
            KeyCode::Enter => self.enter_file(),
            KeyCode::Backspace => {
                self.state.change_dir_in_parent_dir();
                Ok(Action::None)
            }
            KeyCode::Char('<') => Ok(Action::NavigateBack),
            KeyCode::Char('>') => Ok(Action::NavigateForward),
            KeyCode::Char('q') => Ok(Action::Quit),
            KeyCode::Char('c') => Ok(self.prompt_copy()),
            KeyCode::Char('d') => Ok(self.prompt_delete()),
            KeyCode::Char('k') => Ok(self.prompt_mkdir()),
            KeyCode::Char('n') => Ok(self.prompt_touch()),
            KeyCode::Char('p') => Ok(self.prompt_zip()),
            KeyCode::Char('u') => Ok(self.prompt_unzip()),
            KeyCode::Char('m') => Ok(self.prompt_move()),
            KeyCode::Char('r') => Ok(self.prompt_rename()),
            KeyCode::Char('s') => Ok(self.prompt_sort()),
            KeyCode::Char('f') => Ok(self.prompt_search()),
            KeyCode::Char('g') => Ok(self.prompt_grep()),
            KeyCode::Char('j') => Ok(self.prompt_jump()),
            KeyCode::Char('h') => Ok(Action::LaunchShell),
            KeyCode::Char(' ') => {
                self.state.toggle_checked_file();
                self.state.next();
                Ok(Action::None)
            }
            KeyCode::Char('.') => {
                self.state.toggle_show_dot_file();
                Ok(Action::None)
            }
            KeyCode::Char('+') => {
                if let Some(file) = self.state.selected_file() {
                    Ok(Action::AddBookmark(file.absolute_path().to_string()))
                } else {
                    Ok(Action::None)
                }
            }
            KeyCode::Char('-') => {
                if let Some(file) = self.state.selected_file() {
                    Ok(Action::RemoveBookmark(file.absolute_path().to_string()))
                } else {
                    Ok(Action::None)
                }
            }
            KeyCode::Char('a') => self.show_attribute(),
            KeyCode::Char('b') => Ok(Action::ShowBookmark),
            KeyCode::Char('i') => self.show_file_info(),
            KeyCode::Char('o') => Ok(Action::ShowSettings),
            KeyCode::Char('t') => Ok(self.show_tree()),
            KeyCode::Char('v') => self.show_preview(),
            KeyCode::Char('x') => Ok(self.prompt_execute()),
            KeyCode::Char('?') => Ok(Action::ShowSidePanel(Box::new(SidePanel::Help(
                HelpComponent::new(),
            )))),
            _ => Ok(Action::None),
        }
    }
}

impl FilerComponent {
    /// Store を参照してファイルテーブルを描画する
    pub fn render_with_store(&mut self, frame: &mut Frame, area: Rect, store: &RootStore) {
        let list_size = self.state.current_dir_files.len();
        let title = if self.state.is_loading() {
            format!(
                "{} ({}) {} Loading",
                self.state.current_dir.absolute_path(),
                list_size,
                self.spinner.frame()
            )
        } else {
            format!("{} ({})", self.state.current_dir.absolute_path(), list_size)
        };
        let border_style = if self.active {
            BorderStyle::Active
        } else {
            BorderStyle::Inactive
        };
        let block = build_bordered_block(title.as_str(), border_style);
        let table = self.build_file_table(block, store);
        frame.render_stateful_widget(table, area, &mut self.state.file_table_state);
    }
}

fn format_time(time: Result<VFileTime>) -> String {
    if let Ok(time) = time {
        return time.to_string();
    }
    "____-__-__ --:--:--".to_string()
}
