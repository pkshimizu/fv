use std::path::Path;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

use crate::cmd::command::{AppCommand, Command, FilerCommand, PromptCommand};
use crate::state::PromptMode;
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

    pub fn next(&self, input: &PromptMode) -> Result<Command> {
        match self.rx.recv_timeout(Duration::from_millis(100)) {
            Ok(AppEvent::Key(key)) => {
                if input.is_active() {
                    Ok(Self::prompt_key_to_command(key, input))
                } else {
                    Ok(Self::key_to_command(key))
                }
            }
            Ok(AppEvent::FileChange) => Ok(Command::Filer(FilerCommand::RefreshFiles)),
            Err(_) => Ok(Command::App(AppCommand::None)),
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
            (_, KeyCode::Char('c')) => Command::Filer(FilerCommand::PromptCopy),
            (_, KeyCode::Char('f')) => Command::Filer(FilerCommand::PromptSearch),
            (_, KeyCode::Char('d')) => Command::Filer(FilerCommand::PromptDelete),
            (_, KeyCode::Char('k')) => Command::Filer(FilerCommand::PromptMkdir),
            (_, KeyCode::Char('m')) => Command::Filer(FilerCommand::PromptMove),
            (_, KeyCode::Char('r')) => Command::Filer(FilerCommand::PromptRename),
            (_, KeyCode::Char('s')) => Command::Filer(FilerCommand::PromptSort),
            (_, KeyCode::Char('q')) => Command::App(AppCommand::Quit),
            (_, KeyCode::Char(' ')) => Command::Filer(FilerCommand::ToggleCheckedFile),
            (_, KeyCode::Char('.')) => Command::Filer(FilerCommand::ToggleDotFiles),
            (_, KeyCode::Char('+')) => Command::Filer(FilerCommand::AddBookmark),
            (_, KeyCode::Char('-')) => Command::Filer(FilerCommand::RemoveBookmark),
            (_, KeyCode::Char('b')) => Command::Filer(FilerCommand::ToggleBookmark),
            (_, KeyCode::Up) => Command::Filer(FilerCommand::MoveCursorUp),
            (_, KeyCode::Down) => Command::Filer(FilerCommand::MoveCursorDown),
            (_, KeyCode::Left) => Command::Filer(FilerCommand::MoveCursorLeft),
            (_, KeyCode::Right) => Command::Filer(FilerCommand::MoveCursorRight),
            (_, KeyCode::Enter) => Command::Filer(FilerCommand::EnterFile),
            (_, KeyCode::Backspace) => Command::Filer(FilerCommand::ChangeParentDir),
            _ => Command::App(AppCommand::None),
        }
    }

    fn prompt_key_to_command(key: KeyEvent, input: &PromptMode) -> Command {
        match input {
            PromptMode::Text { .. } => match key.code {
                KeyCode::Char(c) => Command::Prompt(PromptCommand::Char(c)),
                KeyCode::Backspace => Command::Prompt(PromptCommand::Backspace),
                KeyCode::Enter => Command::Prompt(PromptCommand::Ok),
                KeyCode::Esc => Command::Prompt(PromptCommand::Cancel),
                _ => Command::App(AppCommand::None),
            },
            PromptMode::File { .. } => match key.code {
                KeyCode::Char(c) => Command::Prompt(PromptCommand::Char(c)),
                KeyCode::Backspace => Command::Prompt(PromptCommand::Backspace),
                KeyCode::Tab => Command::Prompt(PromptCommand::Tab),
                KeyCode::Enter => Command::Prompt(PromptCommand::Ok),
                KeyCode::Esc => Command::Prompt(PromptCommand::Cancel),
                _ => Command::App(AppCommand::None),
            },
            PromptMode::Select { .. } => match key.code {
                KeyCode::Left => Command::Prompt(PromptCommand::SelectLeft),
                KeyCode::Right => Command::Prompt(PromptCommand::SelectRight),
                KeyCode::Enter => Command::Prompt(PromptCommand::Ok),
                KeyCode::Esc => Command::Prompt(PromptCommand::Cancel),
                _ => Command::App(AppCommand::None),
            },
            PromptMode::Confirm { .. } => match key.code {
                KeyCode::Char('y') | KeyCode::Enter => Command::Prompt(PromptCommand::Ok),
                KeyCode::Char('n') | KeyCode::Esc => Command::Prompt(PromptCommand::Cancel),
                _ => Command::App(AppCommand::None),
            },
            PromptMode::Search { .. } => match key.code {
                KeyCode::Char(c) => Command::Prompt(PromptCommand::Char(c)),
                KeyCode::Backspace => Command::Prompt(PromptCommand::Backspace),
                KeyCode::Down => Command::Prompt(PromptCommand::SearchNext),
                KeyCode::Up => Command::Prompt(PromptCommand::SearchPrev),
                KeyCode::Enter => Command::Prompt(PromptCommand::Ok),
                KeyCode::Esc => Command::Prompt(PromptCommand::Cancel),
                _ => Command::App(AppCommand::None),
            },
            PromptMode::Error { .. } => match key.code {
                KeyCode::Enter | KeyCode::Esc => Command::Prompt(PromptCommand::Cancel),
                _ => Command::App(AppCommand::None),
            },
            PromptMode::None => Command::App(AppCommand::None),
        }
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new(Duration::from_millis(250))
    }
}
