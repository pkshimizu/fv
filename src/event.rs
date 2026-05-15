use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

use crate::cmd::command::{AppCommand, Command, FilerCommand};
use crate::state::{AppState, Area};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

pub enum AppEvent {
    Key(KeyEvent),
    FileChange,
}

/// EventHandler::next の戻り値
pub enum AppEventResult {
    /// 何もない（タイムアウト）
    None,
    /// 既存の Command ベースの処理
    Command(Command),
    /// コンポーネントに委譲するキーイベント
    KeyEvent(KeyEvent),
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

    pub fn next(&self, state: &AppState) -> Result<AppEventResult> {
        match self.rx.recv_timeout(Duration::from_millis(100)) {
            Ok(AppEvent::Key(key)) => Ok(match state.active_area() {
                Area::SideComponent | Area::Prompt => AppEventResult::KeyEvent(key),
                Area::Filer => AppEventResult::Command(Self::key_to_command(key)),
            }),
            Ok(AppEvent::FileChange) => Ok(AppEventResult::Command(Command::Filer(
                FilerCommand::RefreshFiles,
            ))),
            Err(_) => Ok(AppEventResult::None),
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
            (_, KeyCode::Char('n')) => Command::Filer(FilerCommand::PromptTouch),
            (_, KeyCode::Char('p')) => Command::Filer(FilerCommand::PromptZip),
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
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new(Duration::from_millis(250))
    }
}
