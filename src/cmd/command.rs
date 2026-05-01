use crate::cmd::{app, file, filer, input};
use crate::state::AppState;
use anyhow::Result;

pub enum FilerCommand {
    MoveCursorUp,
    MoveCursorDown,
    MoveCursorLeft,
    MoveCursorRight,
    EnterFile,
    ChangeParentDir,
    Copy,
    Delete,
    Mkdir,
    Move,
    Rename,
    Sort,
    Search,
    RefreshFiles,
    ToggleCheckedFile,
    ToggleDotFiles,
}

impl FilerCommand {
    pub fn exec(self, state: &mut AppState) -> Result<()> {
        match self {
            FilerCommand::MoveCursorUp => filer::up_cursor(state),
            FilerCommand::MoveCursorDown => filer::down_cursor(state),
            FilerCommand::MoveCursorLeft => filer::first_cursor(state),
            FilerCommand::MoveCursorRight => filer::last_cursor(state),
            FilerCommand::EnterFile => file::enter_file(state),
            FilerCommand::ChangeParentDir => filer::change_to_parent(state),
            FilerCommand::Copy => filer::copy(state),
            FilerCommand::Delete => filer::delete(state),
            FilerCommand::Mkdir => filer::mkdir(state),
            FilerCommand::Move => filer::move_to(state),
            FilerCommand::Rename => filer::rename(state),
            FilerCommand::Sort => filer::sort(state),
            FilerCommand::Search => filer::search(state),
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

impl InputAreaCommand {
    pub fn exec(self, state: &mut AppState) -> Result<()> {
        match self {
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

pub enum Command {
    Filer(FilerCommand),
    InputArea(InputAreaCommand),
    Quit,
    None,
}

impl Command {
    pub fn exec(self, state: &mut AppState) -> Result<()> {
        match self {
            Command::Filer(cmd) => cmd.exec(state),
            Command::InputArea(cmd) => cmd.exec(state),
            Command::Quit => app::quit(state),
            Command::None => Ok(()),
        }
    }
}
