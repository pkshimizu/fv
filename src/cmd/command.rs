use crate::cmd::{change_dir, move_cursor, quit, refresh_files};
use crate::state::AppState;

pub enum Command {
    MoveCursorUp,
    MoveCursorDown,
    MoveCursorLeft,
    MoveCursorRight,
    ChangeDir,
    ChangeParentDir,
    RefreshFiles,
    Quit,
    None,
}

impl Command {
    pub fn exec(self, state: &mut AppState) {
        match self {
            Command::MoveCursorUp => move_cursor::up(state),
            Command::MoveCursorDown => move_cursor::down(state),
            Command::MoveCursorLeft => move_cursor::first(state),
            Command::MoveCursorRight => move_cursor::last(state),
            Command::ChangeDir => change_dir::select_dir(state),
            Command::ChangeParentDir => change_dir::parent_dir(state),
            Command::RefreshFiles => refresh_files::exec(state),
            Command::Quit => quit::exec(state),
            Command::None => {}
        }
    }
}
