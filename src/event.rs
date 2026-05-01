use std::path::Path;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

use crate::cmd::command::{AppCommand, Executable, FilerCommand, PromptCommand};
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

    pub fn next(&self, input: &InputMode) -> Result<Box<dyn Executable>> {
        match self.rx.recv_timeout(Duration::from_millis(100)) {
            Ok(AppEvent::Key(key)) => {
                if input.is_active() {
                    Ok(Self::prompt_key_to_executable(key, input))
                } else {
                    Ok(Self::key_to_executable(key))
                }
            }
            Ok(AppEvent::FileChange) => Ok(Box::new(FilerCommand::RefreshFiles)),
            Err(_) => Ok(Box::new(AppCommand::None)),
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

    fn key_to_executable(key: KeyEvent) -> Box<dyn Executable> {
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('c')) => Box::new(FilerCommand::PromptCopy),
            (_, KeyCode::Char('f')) => Box::new(FilerCommand::PromptSearch),
            (_, KeyCode::Char('d')) => Box::new(FilerCommand::PromptDelete),
            (_, KeyCode::Char('k')) => Box::new(FilerCommand::PromptMkdir),
            (_, KeyCode::Char('m')) => Box::new(FilerCommand::PromptMove),
            (_, KeyCode::Char('r')) => Box::new(FilerCommand::PromptRename),
            (_, KeyCode::Char('s')) => Box::new(FilerCommand::PromptSort),
            (_, KeyCode::Char('q')) => Box::new(AppCommand::Quit),
            (_, KeyCode::Char(' ')) => Box::new(FilerCommand::ToggleCheckedFile),
            (_, KeyCode::Char('.')) => Box::new(FilerCommand::ToggleDotFiles),
            (_, KeyCode::Up) => Box::new(FilerCommand::MoveCursorUp),
            (_, KeyCode::Down) => Box::new(FilerCommand::MoveCursorDown),
            (_, KeyCode::Left) => Box::new(FilerCommand::MoveCursorLeft),
            (_, KeyCode::Right) => Box::new(FilerCommand::MoveCursorRight),
            (_, KeyCode::Enter) => Box::new(FilerCommand::EnterFile),
            (_, KeyCode::Backspace) => Box::new(FilerCommand::ChangeParentDir),
            _ => Box::new(AppCommand::None),
        }
    }

    fn prompt_key_to_executable(key: KeyEvent, input: &InputMode) -> Box<dyn Executable> {
        match input {
            InputMode::Text { .. } => match key.code {
                KeyCode::Char(c) => Box::new(PromptCommand::Char(c)),
                KeyCode::Backspace => Box::new(PromptCommand::Backspace),
                KeyCode::Enter => Box::new(PromptCommand::Ok),
                KeyCode::Esc => Box::new(PromptCommand::Cancel),
                _ => Box::new(AppCommand::None),
            },
            InputMode::File { .. } => match key.code {
                KeyCode::Char(c) => Box::new(PromptCommand::Char(c)),
                KeyCode::Backspace => Box::new(PromptCommand::Backspace),
                KeyCode::Tab => Box::new(PromptCommand::Tab),
                KeyCode::Enter => Box::new(PromptCommand::Ok),
                KeyCode::Esc => Box::new(PromptCommand::Cancel),
                _ => Box::new(AppCommand::None),
            },
            InputMode::Select { .. } => match key.code {
                KeyCode::Left => Box::new(PromptCommand::SelectLeft),
                KeyCode::Right => Box::new(PromptCommand::SelectRight),
                KeyCode::Enter => Box::new(PromptCommand::Ok),
                KeyCode::Esc => Box::new(PromptCommand::Cancel),
                _ => Box::new(AppCommand::None),
            },
            InputMode::Confirm { .. } => match key.code {
                KeyCode::Char('y') | KeyCode::Enter => Box::new(PromptCommand::Ok),
                KeyCode::Char('n') | KeyCode::Esc => Box::new(PromptCommand::Cancel),
                _ => Box::new(AppCommand::None),
            },
            InputMode::Search { .. } => match key.code {
                KeyCode::Char(c) => Box::new(PromptCommand::Char(c)),
                KeyCode::Backspace => Box::new(PromptCommand::Backspace),
                KeyCode::Down => Box::new(PromptCommand::SearchNext),
                KeyCode::Up => Box::new(PromptCommand::SearchPrev),
                KeyCode::Enter => Box::new(PromptCommand::Ok),
                KeyCode::Esc => Box::new(PromptCommand::Cancel),
                _ => Box::new(AppCommand::None),
            },
            InputMode::Error { .. } => match key.code {
                KeyCode::Enter | KeyCode::Esc => Box::new(PromptCommand::Cancel),
                _ => Box::new(AppCommand::None),
            },
            InputMode::None => Box::new(AppCommand::None),
        }
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new(Duration::from_millis(250))
    }
}
