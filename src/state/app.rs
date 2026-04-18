use crate::config::Config;
use crate::state::FilerState;
use crate::state::InputMode;
use anyhow::Result;

#[derive(Debug)]
pub struct AppState {
    pub config: Config,
    pub running: bool,
    pub filer: FilerState,
    pub input: InputMode,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let filer_state = FilerState::new();
        Self {
            config,
            running: true,
            filer: filer_state,
            input: InputMode::None,
        }
    }

    pub fn init(&mut self) -> Result<()> {
        self.filer.init()?;
        Ok(())
    }

    pub fn quit(&mut self) {
        self.running = false;
    }
}
