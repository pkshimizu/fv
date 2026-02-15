use crate::config::Config;

#[derive(Debug)]
pub struct AppState {
    pub config: Config,
    pub running: bool,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            running: true,
        }
    }
}
