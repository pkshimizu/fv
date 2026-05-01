use crate::cmd::{app, file, filer, input};
use crate::state::AppState;
use anyhow::Result;

pub trait Executable {
    fn exec(self: Box<Self>, state: &mut AppState) -> Result<()>;
}

pub enum AppCommand {
    Quit,
    None,
}

impl Executable for AppCommand {
    fn exec(self: Box<Self>, state: &mut AppState) -> Result<()> {
        match *self {
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
    PromptMove,
    PromptRename,
    PromptSort,
    PromptSearch,
    RefreshFiles,
    ToggleCheckedFile,
    ToggleDotFiles,
}

impl Executable for FilerCommand {
    fn exec(self: Box<Self>, state: &mut AppState) -> Result<()> {
        match *self {
            FilerCommand::MoveCursorUp => filer::up_cursor(state),
            FilerCommand::MoveCursorDown => filer::down_cursor(state),
            FilerCommand::MoveCursorLeft => filer::first_cursor(state),
            FilerCommand::MoveCursorRight => filer::last_cursor(state),
            FilerCommand::EnterFile => file::enter_file(state),
            FilerCommand::ChangeParentDir => filer::change_to_parent(state),
            FilerCommand::PromptCopy => filer::prompt_copy(state),
            FilerCommand::PromptDelete => filer::prompt_delete(state),
            FilerCommand::PromptMkdir => filer::prompt_mkdir(state),
            FilerCommand::PromptMove => filer::prompt_move(state),
            FilerCommand::PromptRename => filer::prompt_rename(state),
            FilerCommand::PromptSort => filer::prompt_sort(state),
            FilerCommand::PromptSearch => filer::prompt_search(state),
            FilerCommand::RefreshFiles => filer::refresh_files(state),
            FilerCommand::ToggleCheckedFile => filer::toggle_checked_file(state),
            FilerCommand::ToggleDotFiles => filer::toggle_dot_files(state),
        }
    }
}

pub enum InputAreaCommand {
    Char(char),
    Backspace,
    Tab,
    SelectLeft,
    SelectRight,
    Ok,
    Cancel,
    SearchNext,
    SearchPrev,
}

impl Executable for InputAreaCommand {
    fn exec(self: Box<Self>, state: &mut AppState) -> Result<()> {
        match *self {
            InputAreaCommand::Char(c) => input::input_char(state, c),
            InputAreaCommand::Backspace => input::input_backspace(state),
            InputAreaCommand::Tab => input::input_tab(state),
            InputAreaCommand::SelectLeft => input::input_select_left(state),
            InputAreaCommand::SelectRight => input::input_select_right(state),
            InputAreaCommand::Ok => input::input_ok(state),
            InputAreaCommand::Cancel => input::input_cancel(state),
            InputAreaCommand::SearchNext => input::input_search_next(state),
            InputAreaCommand::SearchPrev => input::input_search_prev(state),
        }
    }
}
