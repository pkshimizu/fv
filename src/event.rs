use std::io;
use std::time::Duration;

use crate::cmd::command::Command;
use crossterm::event::{self, Event, KeyCode, KeyEvent};

pub struct EventHandler {
    tick_rate: Duration,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    pub fn next(&self) -> io::Result<Command> {
        if event::poll(self.tick_rate)? {
            if let Event::Key(key_event) = event::read()? {
                return Ok(Self::key_to_command(key_event));
            }
        }
        Ok(Command::None)
    }

    fn key_to_command(key: KeyEvent) -> Command {
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q')) => Command::Quit,
            (_, KeyCode::Up) => Command::FilerCursorUp,
            (_, KeyCode::Down) => Command::FilerCursorDown,
            (_, KeyCode::Left) => Command::FilerCursorLeft,
            (_, KeyCode::Right) => Command::FilerCursorRight,
            _ => Command::None,
        }
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new(Duration::from_millis(250))
    }
}
