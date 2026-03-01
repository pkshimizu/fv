use crate::cmd::{change_dir, filer_cursor, quit};
use crate::state::AppState;

pub enum Command {
    FilerCursorUp,
    FilerCursorDown,
    FilerCursorLeft,
    FilerCursorRight,
    ChangeDir,
    Quit,
    None,
}

impl Command {
    pub fn exec(self, state: &mut AppState) {
        match self {
            Command::FilerCursorUp => filer_cursor::up(state),
            Command::FilerCursorDown => filer_cursor::down(state),
            Command::FilerCursorLeft => filer_cursor::first(state),
            Command::FilerCursorRight => filer_cursor::last(state),
            Command::ChangeDir => change_dir::exec(state),
            Command::Quit => quit::exec(state),
            Command::None => {}
        }
    }
}
