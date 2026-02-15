use crate::cmd::quit;
use crate::state::AppState;

pub enum Command {
    Quit,
    None,
}

impl Command {
    pub fn exec(self, state: &mut AppState) {
        match self {
            Command::Quit => quit::exec(state),
            Command::None => {}
        }
    }
}
