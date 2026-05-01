use std::path::Path;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

use crate::cmd::command::Command;
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

    pub fn next(&self, input: &InputMode, bookmark_list_active: bool) -> Result<Command> {
        match self.rx.recv_timeout(Duration::from_millis(100)) {
            Ok(AppEvent::Key(key)) => {
                if input.is_active() {
                    Ok(Self::input_key_to_command(key, input))
                } else if bookmark_list_active {
                    Ok(Self::bookmark_list_key_to_command(key))
                } else {
                    Ok(Self::key_to_command(key))
                }
            }
            Ok(AppEvent::FileChange) => Ok(Command::RefreshFiles),
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
            (_, KeyCode::Char('c')) => Command::InputCopy,
            (_, KeyCode::Char('f')) => Command::InputSearch,
            (_, KeyCode::Char('d')) => Command::InputDelete,
            (_, KeyCode::Char('k')) => Command::InputMkdir,
            (_, KeyCode::Char('m')) => Command::InputMove,
            (_, KeyCode::Char('r')) => Command::InputRename,
            (_, KeyCode::Char('s')) => Command::InputSort,
            (_, KeyCode::Char('b')) => Command::ShowBookmarkList,
            (_, KeyCode::Char('q')) => Command::Quit,
            (_, KeyCode::Char(' ')) => Command::ToggleCheckedFile,
            (_, KeyCode::Char('.')) => Command::ToggleDotFiles,
            (_, KeyCode::Char('+')) => Command::AddBookmark,
            (_, KeyCode::Char('-')) => Command::RemoveBookmark,
            (_, KeyCode::Up) => Command::MoveCursorUp,
            (_, KeyCode::Down) => Command::MoveCursorDown,
            (_, KeyCode::Left) => Command::MoveCursorLeft,
            (_, KeyCode::Right) => Command::MoveCursorRight,
            (_, KeyCode::Enter) => Command::EnterFile,
            (_, KeyCode::Backspace) => Command::ChangeParentDir,
            _ => Command::None,
        }
    }

    fn bookmark_list_key_to_command(key: KeyEvent) -> Command {
        match key.code {
            KeyCode::Up => Command::BookmarkCursorUp,
            KeyCode::Down => Command::BookmarkCursorDown,
            KeyCode::Esc | KeyCode::Char('b') => Command::CloseBookmarkList,
            _ => Command::None,
        }
    }

    fn input_key_to_command(key: KeyEvent, input: &InputMode) -> Command {
        match input {
            InputMode::Text { .. } => match key.code {
                KeyCode::Char(c) => Command::InputChar(c),
                KeyCode::Backspace => Command::InputBackspace,
                KeyCode::Enter => Command::InputOk,
                KeyCode::Esc => Command::InputCancel,
                _ => Command::None,
            },
            InputMode::File { .. } => match key.code {
                KeyCode::Char(c) => Command::InputChar(c),
                KeyCode::Backspace => Command::InputBackspace,
                KeyCode::Tab => Command::InputTab,
                KeyCode::Enter => Command::InputOk,
                KeyCode::Esc => Command::InputCancel,
                _ => Command::None,
            },
            InputMode::Select { .. } => match key.code {
                KeyCode::Left => Command::InputSelectLeft,
                KeyCode::Right => Command::InputSelectRight,
                KeyCode::Enter => Command::InputOk,
                KeyCode::Esc => Command::InputCancel,
                _ => Command::None,
            },
            InputMode::Confirm { .. } => match key.code {
                KeyCode::Char('y') | KeyCode::Enter => Command::InputOk,
                KeyCode::Char('n') | KeyCode::Esc => Command::InputCancel,
                _ => Command::None,
            },
            InputMode::Search { .. } => match key.code {
                KeyCode::Char(c) => Command::InputChar(c),
                KeyCode::Backspace => Command::InputBackspace,
                KeyCode::Down => Command::InputSearchNext,
                KeyCode::Up => Command::InputSearchPrev,
                KeyCode::Enter => Command::InputOk,
                KeyCode::Esc => Command::InputCancel,
                _ => Command::None,
            },
            InputMode::Error { .. } => match key.code {
                KeyCode::Enter | KeyCode::Esc => Command::InputCancel,
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
