use crate::cmd::{app, file, filer, input};
use crate::state::AppState;
use anyhow::Result;

pub enum Command {
    MoveCursorUp,
    MoveCursorDown,
    MoveCursorLeft,
    MoveCursorRight,
    EnterFile,
    ChangeParentDir,
    RefreshFiles,
    ToggleCheckedFile,
    OpenDeleteConfirm,
    InputChar(char),
    InputBackspace,
    InputOk,
    InputCancel,
    Quit,
    None,
}

impl Command {
    pub fn exec(self, state: &mut AppState) -> Result<()> {
        match self {
            Command::MoveCursorUp => filer::up_cursor(state),
            Command::MoveCursorDown => filer::down_cursor(state),
            Command::MoveCursorLeft => filer::first_cursor(state),
            Command::MoveCursorRight => filer::last_cursor(state),
            Command::EnterFile => file::enter_file(state),
            Command::ChangeParentDir => filer::change_to_parent(state),
            Command::RefreshFiles => filer::refresh_files(state),
            Command::ToggleCheckedFile => filer::toggle_checked_file(state),
            Command::OpenDeleteConfirm => input::open_delete_confirm(state),
            Command::InputChar(c) => input::input_char(state, c),
            Command::InputBackspace => input::input_backspace(state),
            Command::InputOk => input::input_ok(state),
            Command::InputCancel => input::input_cancel(state),
            Command::Quit => app::quit(state),
            Command::None => Ok(()),
        }
    }
}
