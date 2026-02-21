use crate::cmd::{filer_cursor, quit};
use crate::state::AppState;

pub enum Command {
    FilerCursorUp,
    FilerCursorDown,
    FilerCursorLeft,
    FilerCursorRight,
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
            Command::Quit => quit::exec(state),
            Command::None => {}
        }
    }
}
