use crate::cmd::{app, attribute, bookmark, file_info, filer, grep, prompt};
use crate::state::AppState;
use crate::store::RootStore;
use anyhow::Result;

pub enum Command {
    App(AppCommand),
    Filer(FilerCommand),
    Prompt(PromptCommand),
    Bookmark(BookmarkCommand),
    Grep(GrepCommand),
}

impl Command {
    pub fn exec(self, state: &mut AppState, store: &mut RootStore) -> Result<()> {
        match self {
            Command::App(cmd) => cmd.exec(state),
            Command::Filer(cmd) => cmd.exec(state, store),
            Command::Prompt(cmd) => cmd.exec(state),
            Command::Bookmark(cmd) => cmd.exec(state, store),
            Command::Grep(cmd) => cmd.exec(state),
        }
    }
}

pub enum AppCommand {
    Quit,
    None,
}

impl AppCommand {
    fn exec(self, state: &mut AppState) -> Result<()> {
        match self {
            AppCommand::Quit => app::quit(state),
            AppCommand::None => Ok(()),
        }
    }
}

pub enum FilerCommand {
    MoveCursorUp,
    MoveCursorDown,
    MoveCursorLeft,
    MoveCursorRight,
    EnterFile,
    ChangeParentDir,
    PromptCopy,
    PromptDelete,
    PromptMkdir,
    PromptTouch,
    PromptZip,
    PromptMove,
    PromptRename,
    PromptSort,
    PromptSearch,
    PromptGrep,
    PromptJump,
    AddBookmark,
    RemoveBookmark,
    ShowBookmark,
    RefreshFiles,
    ToggleCheckedFile,
    ShowAttribute,
    ShowFileInfo,
    ToggleDotFiles,
    LaunchShell,
}

impl FilerCommand {
    fn exec(self, state: &mut AppState, store: &mut RootStore) -> Result<()> {
        match self {
            FilerCommand::MoveCursorUp => filer::up_cursor(state),
            FilerCommand::MoveCursorDown => filer::down_cursor(state),
            FilerCommand::MoveCursorLeft => filer::first_cursor(state),
            FilerCommand::MoveCursorRight => filer::last_cursor(state),
            FilerCommand::EnterFile => filer::enter_file(state),
            FilerCommand::ChangeParentDir => filer::change_to_parent(state),
            FilerCommand::PromptCopy => filer::prompt_copy(state),
            FilerCommand::PromptDelete => filer::prompt_delete(state),
            FilerCommand::PromptMkdir => filer::prompt_mkdir(state),
            FilerCommand::PromptTouch => filer::prompt_touch(state),
            FilerCommand::PromptZip => filer::prompt_zip(state),
            FilerCommand::PromptMove => filer::prompt_move(state),
            FilerCommand::PromptRename => filer::prompt_rename(state),
            FilerCommand::PromptSort => filer::prompt_sort(state),
            FilerCommand::PromptSearch => filer::prompt_search(state),
            FilerCommand::PromptGrep => filer::prompt_grep(state),
            FilerCommand::LaunchShell => filer::launch_shell(state),
            FilerCommand::PromptJump => filer::prompt_jump(state),
            FilerCommand::AddBookmark => filer::add_bookmark(state, store),
            FilerCommand::RemoveBookmark => filer::remove_bookmark(state, store),
            FilerCommand::ShowBookmark => filer::show_bookmark(state, store),
            FilerCommand::RefreshFiles => filer::refresh_files(state),
            FilerCommand::ToggleCheckedFile => filer::toggle_checked_file(state),
            FilerCommand::ShowAttribute => attribute::show_attribute(state),
            FilerCommand::ShowFileInfo => file_info::show_file_info(state),
            FilerCommand::ToggleDotFiles => filer::toggle_dot_files(state),
        }
    }
}

pub enum PromptCommand {
    Char(char),
    Backspace,
    CursorLeft,
    CursorRight,
    Tab,
    BackTab,
    SelectLeft,
    SelectRight,
    Ok,
    Cancel,
    SearchNext,
    SearchPrev,
}

impl PromptCommand {
    fn exec(self, state: &mut AppState) -> Result<()> {
        match self {
            PromptCommand::Char(c) => prompt::input_char(state, c),
            PromptCommand::Backspace => prompt::input_backspace(state),
            PromptCommand::CursorLeft => prompt::input_cursor_left(state),
            PromptCommand::CursorRight => prompt::input_cursor_right(state),
            PromptCommand::Tab => prompt::input_tab(state),
            PromptCommand::BackTab => prompt::input_back_tab(state),
            PromptCommand::SelectLeft => prompt::input_select_left(state),
            PromptCommand::SelectRight => prompt::input_select_right(state),
            PromptCommand::Ok => prompt::input_ok(state),
            PromptCommand::Cancel => prompt::input_cancel(state),
            PromptCommand::SearchNext => prompt::input_search_next(state),
            PromptCommand::SearchPrev => prompt::input_search_prev(state),
        }
    }
}

pub enum BookmarkCommand {
    MoveCursorUp,
    MoveCursorDown,
    MoveCursorLeft,
    MoveCursorRight,
    EnterFile,
    RemoveBookmark,
    HideBookmark,
}

impl BookmarkCommand {
    fn exec(self, state: &mut AppState, store: &mut RootStore) -> Result<()> {
        match self {
            BookmarkCommand::MoveCursorUp => bookmark::up_cursor(state),
            BookmarkCommand::MoveCursorDown => bookmark::down_cursor(state),
            BookmarkCommand::MoveCursorLeft => bookmark::first_cursor(state),
            BookmarkCommand::MoveCursorRight => bookmark::last_cursor(state),
            BookmarkCommand::EnterFile => bookmark::select(state),
            BookmarkCommand::RemoveBookmark => bookmark::remove_bookmark(state, store),
            BookmarkCommand::HideBookmark => bookmark::hide_bookmark(state),
        }
    }
}

pub enum GrepCommand {
    MoveCursorUp,
    MoveCursorDown,
    MoveCursorLeft,
    MoveCursorRight,
    EnterFile,
    HideGrep,
}

impl GrepCommand {
    fn exec(self, state: &mut AppState) -> Result<()> {
        match self {
            GrepCommand::MoveCursorUp => grep::up_cursor(state),
            GrepCommand::MoveCursorDown => grep::down_cursor(state),
            GrepCommand::MoveCursorLeft => grep::first_cursor(state),
            GrepCommand::MoveCursorRight => grep::last_cursor(state),
            GrepCommand::EnterFile => grep::select(state),
            GrepCommand::HideGrep => grep::hide_grep(state),
        }
    }
}
