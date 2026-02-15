use std::io;

use ratatui::DefaultTerminal;

use crate::config::Config;
use crate::event::EventHandler;
use crate::state::AppState;
use crate::ui;

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

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while self.state.running {
            // UI を描画
            terminal.draw(|frame| ui::render_main_view(frame, &self.state))?;

            // イベントを取得してコマンドに変換
            let command = self.event_handler.next()?;
            command.exec(&mut self.state);
        }
        Ok(())
    }
}
