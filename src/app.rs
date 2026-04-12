use ratatui::DefaultTerminal;

use crate::config::Config;
use crate::event::EventHandler;
use crate::state::AppState;
use crate::ui;
use anyhow::Result;

pub struct App {
    state: AppState,
    event_handler: EventHandler,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            state: AppState::new(config),
            event_handler: EventHandler::default(),
        }
    }

    pub fn init(&mut self) -> Result<()> {
        self.state.init()?;
        Ok(())
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let mut watching_dir_path = self.state.filer.current_dir.absolute_path().to_string();

        while self.state.running {
            // UI を描画
            terminal.draw(|frame| ui::render_main_view(frame, &mut self.state))?;

            // イベントを取得してコマンドに変換
            let command = self.event_handler.next(&self.state.modal)?;
            command.exec(&mut self.state)?;

            // カレントディレクトリの監視
            let current_dir_path = self.state.filer.current_dir.absolute_path();
            if current_dir_path != watching_dir_path {
                self.event_handler.watch_directory(&current_dir_path)?;
                watching_dir_path = current_dir_path.to_string();
            }
        }
        Ok(())
    }
}
