use std::path::Path;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

use crate::cmd::command::{Command, FilerCommand, InputAreaCommand};
use crate::state::InputMode;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

pub enum AppEvent {
    Key(KeyEvent),
    FileChange,
}

pub struct EventHandler {
    rx: Receiver<AppEvent>,
    tx: Sender<AppEvent>,
    watcher: Option<RecommendedWatcher>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::channel();
        let key_tx = tx.clone();

        thread::spawn(move || {
            loop {
                if event::poll(tick_rate).unwrap_or(false) {
                    if let Ok(Event::Key(event)) = event::read() {
                        if key_tx.send(AppEvent::Key(event)).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Self {
            rx,
            tx,
            watcher: None,
        }
    }

    pub fn next(&self, input: &InputMode) -> Result<Command> {
        match self.rx.recv_timeout(Duration::from_millis(100)) {
            Ok(AppEvent::Key(key)) => {
                if input.is_active() {
                    Ok(Self::input_key_to_command(key, input))
                } else {
                    Ok(Self::key_to_command(key))
                }
            }
            Ok(AppEvent::FileChange) => Ok(Command::Filer(FilerCommand::RefreshFiles)),
            Err(_) => Ok(Command::None),
        }
    }

    pub fn watch_directory(&mut self, path: &str) -> Result<()> {
        let tx = self.tx.clone();

        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<notify::Event>| {
                if res.is_ok() {
                    let _ = tx.send(AppEvent::FileChange);
                }
            },
            Config::default(),
        )?;

        watcher.watch(Path::new(path), RecursiveMode::NonRecursive)?;
        self.watcher = Some(watcher);
        Ok(())
    }

    fn key_to_command(key: KeyEvent) -> Command {
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('c')) => Command::Filer(FilerCommand::Copy),
            (_, KeyCode::Char('f')) => Command::Filer(FilerCommand::Search),
            (_, KeyCode::Char('d')) => Command::Filer(FilerCommand::Delete),
            (_, KeyCode::Char('k')) => Command::Filer(FilerCommand::Mkdir),
            (_, KeyCode::Char('m')) => Command::Filer(FilerCommand::Move),
            (_, KeyCode::Char('r')) => Command::Filer(FilerCommand::Rename),
            (_, KeyCode::Char('s')) => Command::Filer(FilerCommand::Sort),
            (_, KeyCode::Char('q')) => Command::Quit,
            (_, KeyCode::Char(' ')) => Command::Filer(FilerCommand::ToggleCheckedFile),
            (_, KeyCode::Char('.')) => Command::Filer(FilerCommand::ToggleDotFiles),
            (_, KeyCode::Up) => Command::Filer(FilerCommand::MoveCursorUp),
            (_, KeyCode::Down) => Command::Filer(FilerCommand::MoveCursorDown),
            (_, KeyCode::Left) => Command::Filer(FilerCommand::MoveCursorLeft),
            (_, KeyCode::Right) => Command::Filer(FilerCommand::MoveCursorRight),
            (_, KeyCode::Enter) => Command::Filer(FilerCommand::EnterFile),
            (_, KeyCode::Backspace) => Command::Filer(FilerCommand::ChangeParentDir),
            _ => Command::None,
        }
    }

    fn input_key_to_command(key: KeyEvent, input: &InputMode) -> Command {
        match input {
            InputMode::Text { .. } => match key.code {
                KeyCode::Char(c) => Command::InputArea(InputAreaCommand::Char(c)),
                KeyCode::Backspace => Command::InputArea(InputAreaCommand::Backspace),
                KeyCode::Enter => Command::InputArea(InputAreaCommand::Ok),
                KeyCode::Esc => Command::InputArea(InputAreaCommand::Cancel),
                _ => Command::None,
            },
            InputMode::File { .. } => match key.code {
                KeyCode::Char(c) => Command::InputArea(InputAreaCommand::Char(c)),
                KeyCode::Backspace => Command::InputArea(InputAreaCommand::Backspace),
                KeyCode::Tab => Command::InputArea(InputAreaCommand::Tab),
                KeyCode::Enter => Command::InputArea(InputAreaCommand::Ok),
                KeyCode::Esc => Command::InputArea(InputAreaCommand::Cancel),
                _ => Command::None,
            },
            InputMode::Select { .. } => match key.code {
                KeyCode::Left => Command::InputArea(InputAreaCommand::SelectLeft),
                KeyCode::Right => Command::InputArea(InputAreaCommand::SelectRight),
                KeyCode::Enter => Command::InputArea(InputAreaCommand::Ok),
                KeyCode::Esc => Command::InputArea(InputAreaCommand::Cancel),
                _ => Command::None,
            },
            InputMode::Confirm { .. } => match key.code {
                KeyCode::Char('y') | KeyCode::Enter => Command::InputArea(InputAreaCommand::Ok),
                KeyCode::Char('n') | KeyCode::Esc => Command::InputArea(InputAreaCommand::Cancel),
                _ => Command::None,
            },
            InputMode::Search { .. } => match key.code {
                KeyCode::Char(c) => Command::InputArea(InputAreaCommand::Char(c)),
                KeyCode::Backspace => Command::InputArea(InputAreaCommand::Backspace),
                KeyCode::Down => Command::InputArea(InputAreaCommand::SearchNext),
                KeyCode::Up => Command::InputArea(InputAreaCommand::SearchPrev),
                KeyCode::Enter => Command::InputArea(InputAreaCommand::Ok),
                KeyCode::Esc => Command::InputArea(InputAreaCommand::Cancel),
                _ => Command::None,
            },
            InputMode::Error { .. } => match key.code {
                KeyCode::Enter | KeyCode::Esc => Command::InputArea(InputAreaCommand::Cancel),
                _ => Command::None,
            },
            InputMode::None => Command::None,
        }
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new(Duration::from_millis(250))
    }
}
