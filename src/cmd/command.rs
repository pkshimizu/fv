use crate::cmd::{app, file, filer};
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
            Command::MoveCursorUp => filer::up_cursor(state),
            Command::MoveCursorDown => filer::down_cursor(state),
            Command::MoveCursorLeft => filer::first_cursor(state),
            Command::MoveCursorRight => filer::last_cursor(state),
            Command::EnterFile => file::enter_file(state),
            Command::ChangeParentDir => filer::change_to_parent(state),
            Command::RefreshFiles => filer::refresh_files(state),
            Command::Quit => app::quit(state),
            Command::None => Ok(()),
        }
    }
}
