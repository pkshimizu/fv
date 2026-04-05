use crate::cmd::{change_dir, enter_file, move_cursor, quit, refresh_files};
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
    Quit,
    None,
}

impl Command {
    pub fn exec(self, state: &mut AppState) -> Result<()> {
        match self {
            Command::MoveCursorUp => move_cursor::up(state),
            Command::MoveCursorDown => move_cursor::down(state),
            Command::MoveCursorLeft => move_cursor::first(state),
            Command::MoveCursorRight => move_cursor::last(state),
            Command::EnterFile => enter_file::exec(state),
            Command::ChangeParentDir => change_dir::parent_dir(state),
            Command::RefreshFiles => refresh_files::exec(state),
            Command::Quit => quit::exec(state),
            Command::None => Ok(()),
        }
    }
}
