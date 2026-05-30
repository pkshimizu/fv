use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyEvent};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

enum AppEvent {
    Key(KeyEvent),
    FileChange,
}

/// EventHandler::next_event の戻り値
pub enum InputEvent {
    /// キーイベント
    Key(KeyEvent),
    /// ファイル変更検知
    FileChange,
    /// 何もなし（タイムアウト）
    None,
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
                if event::poll(tick_rate).unwrap_or(false)
                    && let Ok(Event::Key(event)) = event::read()
                    && key_tx.send(AppEvent::Key(event)).is_err()
                {
                    break;
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

    /// 次のイベントを返す
    pub fn next_event(&mut self) -> Result<InputEvent> {
        match self.rx.recv_timeout(Duration::from_millis(100)) {
            Ok(AppEvent::Key(key)) => Ok(InputEvent::Key(key)),
            Ok(AppEvent::FileChange) => Ok(InputEvent::FileChange),
            Err(_) => Ok(InputEvent::None),
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
        )
        .context("Failed to create file watcher")?;

        watcher
            .watch(Path::new(path), RecursiveMode::NonRecursive)
            .with_context(|| format!("{path}: Failed to watch directory"))?;
        self.watcher = Some(watcher);
        Ok(())
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new(Duration::from_millis(250))
    }
}
