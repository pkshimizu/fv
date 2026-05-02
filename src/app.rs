use ratatui::DefaultTerminal;

use crate::config::Config;
use crate::event::EventHandler;
use crate::state::{AppState, InputMode};
use crate::store::RootStore;
use crate::ui;
use anyhow::Result;

pub struct App {
    state: AppState,
    store: RootStore,
    event_handler: EventHandler,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            state: AppState::new(config),
            store: RootStore::new(),
            event_handler: EventHandler::default(),
        }
    }

    pub fn init(&mut self) -> Result<()> {
        self.state.init()?;
        self.store.init()?;
        Ok(())
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let mut watching_dir_path = self.state.filer.current_dir.absolute_path().to_string();

        while self.state.running {
            // UI を描画
            terminal.draw(|frame| ui::render_main_view(frame, &mut self.state, &self.store))?;

            // イベントを取得してコマンドに変換
            let executable = self.event_handler.next(&self.state.input)?;
            if let Err(e) = executable.exec(&mut self.state, &mut self.store) {
                self.state.input = InputMode::Error {
                    message: format!("{e}"),
                };
            }

            // カレントディレクトリの監視
            let current_dir_path = self.state.filer.current_dir.absolute_path();
            if current_dir_path != watching_dir_path {
                self.event_handler.watch_directory(current_dir_path)?;
                watching_dir_path = current_dir_path.to_string();
            }
        }
        Ok(())
    }
}
