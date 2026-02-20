use crate::config::Config;
use crate::state::FilerState;

#[derive(Debug)]
pub struct AppState {
    pub config: Config,
    pub running: bool,
    pub filer: FilerState,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            running: true,
            filer: FilerState::new(),
        }
    }
}
