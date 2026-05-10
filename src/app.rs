use ratatui::DefaultTerminal;

use crate::config::Config;
use crate::event::EventHandler;
use crate::state::{AppState, PromptMode};
use crate::store::RootStore;
use crate::ui;
use anyhow::Result;

pub struct App {
    state: AppState,
    store: RootStore,
    event_handler: EventHandler,
}

impl App {
    pub fn new(config: Config) -> Result<Self> {
        Ok(Self {
            state: AppState::new(config),
            store: RootStore::new()?,
            event_handler: EventHandler::default(),
        })
    }

    pub fn init(&mut self) -> Result<()> {
        self.state.init()?;
        if let Err(e) = self.store.init() {
            tracing::warn!("Failed to initialize bookmark: {}", e);
        }
        Ok(())
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let mut watching_dir_path = self.state.filer.current_dir.absolute_path().to_string();

        while self.state.running {
            // UI を描画
            terminal.draw(|frame| ui::render_main_view(frame, &mut self.state, &self.store))?;

            // イベントを取得してコマンドに変換
            let command = self.event_handler.next(&self.state)?;
            if let Err(e) = command.exec(&mut self.state, &mut self.store) {
                self.state.prompt = PromptMode::Error {
                    message: format!("{e}"),
                };
            }

            // 非同期結果の受信
            self.state.receive_async_results();

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
