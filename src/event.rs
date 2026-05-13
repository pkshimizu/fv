use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::cmd::command::{
    AppCommand, AttributeCommand, BookmarkCommand, Command, FileInfoCommand, FilerCommand,
    GrepCommand, PromptCommand,
};
use crate::state::{AppState, Area, PromptMode};
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
    paused: Arc<AtomicBool>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::channel();
        let key_tx = tx.clone();
        let paused = Arc::new(AtomicBool::new(false));
        let thread_paused = paused.clone();

        thread::spawn(move || {
            loop {
                if thread_paused.load(Ordering::Relaxed) {
                    thread::sleep(tick_rate);
                    continue;
                }
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
            paused,
        }
    }

    pub fn pause(&self) {
        self.paused.store(true, Ordering::Relaxed);
    }

    pub fn resume(&self) {
        self.paused.store(false, Ordering::Relaxed);
    }

    pub fn next(&self, state: &AppState) -> Result<Command> {
        match self.rx.recv_timeout(Duration::from_millis(100)) {
            Ok(AppEvent::Key(key)) => Ok(match state.active_area() {
                Area::Prompt => Self::prompt_key_to_command(key, &state.prompt),
                Area::Attribute => Self::attribute_key_to_command(key),
                Area::Bookmark => Self::bookmark_key_to_command(key),
                Area::Grep => Self::grep_key_to_command(key),
                Area::FileInfo => Self::file_info_key_to_command(key),
                Area::Filer => Self::key_to_command(key),
            }),
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
            (_, KeyCode::Char('g')) => Command::Filer(FilerCommand::PromptGrep),
            (_, KeyCode::Char('d')) => Command::Filer(FilerCommand::PromptDelete),
            (_, KeyCode::Char('k')) => Command::Filer(FilerCommand::PromptMkdir),
            (_, KeyCode::Char('m')) => Command::Filer(FilerCommand::PromptMove),
            (_, KeyCode::Char('r')) => Command::Filer(FilerCommand::PromptRename),
            (_, KeyCode::Char('s')) => Command::Filer(FilerCommand::PromptSort),
            (_, KeyCode::Char('j')) => Command::Filer(FilerCommand::PromptJump),
            (_, KeyCode::Char('q')) => Command::App(AppCommand::Quit),
            (_, KeyCode::Char(' ')) => Command::Filer(FilerCommand::ToggleCheckedFile),
            (_, KeyCode::Char('.')) => Command::Filer(FilerCommand::ToggleDotFiles),
            (_, KeyCode::Char('+')) => Command::Filer(FilerCommand::AddBookmark),
            (_, KeyCode::Char('-')) => Command::Filer(FilerCommand::RemoveBookmark),
            (_, KeyCode::Char('a')) => Command::Filer(FilerCommand::ShowAttribute),
            (_, KeyCode::Char('b')) => Command::Filer(FilerCommand::ShowBookmark),
            (_, KeyCode::Char('h')) => Command::Filer(FilerCommand::LaunchShell),
            (_, KeyCode::Char('i')) => Command::Filer(FilerCommand::ShowFileInfo),
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
                KeyCode::BackTab => Command::Prompt(PromptCommand::BackTab),
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

    fn bookmark_key_to_command(key: KeyEvent) -> Command {
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('-')) => Command::Bookmark(BookmarkCommand::RemoveBookmark),
            (_, KeyCode::Char('b')) => Command::Bookmark(BookmarkCommand::HideBookmark),
            (_, KeyCode::Esc) => Command::Bookmark(BookmarkCommand::HideBookmark),
            (_, KeyCode::Up) => Command::Bookmark(BookmarkCommand::MoveCursorUp),
            (_, KeyCode::Down) => Command::Bookmark(BookmarkCommand::MoveCursorDown),
            (_, KeyCode::Left) => Command::Bookmark(BookmarkCommand::MoveCursorLeft),
            (_, KeyCode::Right) => Command::Bookmark(BookmarkCommand::MoveCursorRight),
            (_, KeyCode::Enter) => Command::Bookmark(BookmarkCommand::EnterFile),
            _ => Command::App(AppCommand::None),
        }
    }

    fn grep_key_to_command(key: KeyEvent) -> Command {
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('g')) => Command::Grep(GrepCommand::HideGrep),
            (_, KeyCode::Esc) => Command::Grep(GrepCommand::HideGrep),
            (_, KeyCode::Up) => Command::Grep(GrepCommand::MoveCursorUp),
            (_, KeyCode::Down) => Command::Grep(GrepCommand::MoveCursorDown),
            (_, KeyCode::Left) => Command::Grep(GrepCommand::MoveCursorLeft),
            (_, KeyCode::Right) => Command::Grep(GrepCommand::MoveCursorRight),
            (_, KeyCode::Enter) => Command::Grep(GrepCommand::EnterFile),
            _ => Command::App(AppCommand::None),
        }
    }

    fn file_info_key_to_command(key: KeyEvent) -> Command {
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('i')) => Command::FileInfo(FileInfoCommand::HideFileInfo),
            (_, KeyCode::Esc) => Command::FileInfo(FileInfoCommand::HideFileInfo),
            (_, KeyCode::Up) => Command::FileInfo(FileInfoCommand::ScrollUp),
            (_, KeyCode::Down) => Command::FileInfo(FileInfoCommand::ScrollDown),
            (_, KeyCode::Left) => Command::FileInfo(FileInfoCommand::ScrollToTop),
            (_, KeyCode::Right) => Command::FileInfo(FileInfoCommand::ScrollToBottom),
            _ => Command::App(AppCommand::None),
        }
    }

    fn attribute_key_to_command(key: KeyEvent) -> Command {
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('a')) => Command::Attribute(AttributeCommand::HideAttribute),
            (_, KeyCode::Esc) => Command::Attribute(AttributeCommand::HideAttribute),
            (_, KeyCode::Up) => Command::Attribute(AttributeCommand::MoveCursorUp),
            (_, KeyCode::Down) => Command::Attribute(AttributeCommand::MoveCursorDown),
            _ => Command::App(AppCommand::None),
        }
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new(Duration::from_millis(250))
    }
}
