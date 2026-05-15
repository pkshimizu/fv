use ratatui::DefaultTerminal;

use crate::component::Action;
use crate::config::Config;
use crate::event::{AppEventResult, EventHandler};
use crate::state::{AppState, PromptMode};
use crate::store::RootStore;
use crate::ui;
use anyhow::{Context, Result};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use std::io::stdout;

pub struct App {
    state: AppState,
    store: RootStore,
    event_handler: EventHandler,
}

impl App {
    pub fn new(config: Config) -> Result<Self> {
        Ok(Self {
            state: AppState::new(config),
            store: RootStore::new()?,
            event_handler: EventHandler::default(),
        })
    }

    pub fn init(&mut self) -> Result<()> {
        self.state.init()?;
        if let Err(e) = self.store.init() {
            tracing::warn!("Failed to initialize bookmark: {}", e);
        }
        Ok(())
    }

    fn launch_external_shell(
        state: &AppState,
        terminal: &mut DefaultTerminal,
        event_handler: &EventHandler,
    ) -> Result<()> {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let dir = state.filer.current_dir.absolute_path();

        // イベントハンドラを一時停止（キー入力の横取りを防止）
        event_handler.pause();

        // TUI を一時停止
        crossterm::terminal::disable_raw_mode()?;
        execute!(stdout(), LeaveAlternateScreen)?;

        // 外部シェルを起動
        let result = std::process::Command::new(&shell)
            .current_dir(dir)
            .status()
            .with_context(|| format!("シェルの起動に失敗しました: {shell}"));

        // TUI を復帰（全ステップを試行し、最初のエラーを返す）
        let r1 = execute!(stdout(), EnterAlternateScreen);
        let r2 = crossterm::terminal::enable_raw_mode();
        let r3 = terminal.clear();

        // エラーの有無に関わらずイベントハンドラを再開
        event_handler.resume();

        r1.and(r2).and(r3)?;

        result?;
        Ok(())
    }

    /// Action を処理する。コンポーネントの handle_event が返した Action をここで実行する。
    fn handle_action(&mut self, action: Action, terminal: &mut DefaultTerminal) -> Result<()> {
        match action {
            Action::None => {}
            Action::Quit => self.state.quit(),
            Action::Error(message) => {
                self.state.prompt = PromptMode::Error { message };
            }
            Action::LaunchShell => {
                if let Err(e) =
                    Self::launch_external_shell(&self.state, terminal, &self.event_handler)
                {
                    self.state.prompt = PromptMode::Error {
                        message: format!("{e}"),
                    };
                }
            }
            Action::CloseSidePanel => {
                self.state.side_panel = None;
            }
        }
        Ok(())
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let mut watching_dir_path = self.state.filer.current_dir.absolute_path().to_string();

        while self.state.running {
            // UI を描画
            terminal.draw(|frame| ui::render_main_view(frame, &mut self.state, &self.store))?;

            // イベントを取得して処理
            match self.event_handler.next(&self.state)? {
                AppEventResult::Command(command) => {
                    if let Err(e) = command.exec(&mut self.state, &mut self.store) {
                        self.state.prompt = PromptMode::Error {
                            message: format!("{e}"),
                        };
                    }
                }
                AppEventResult::KeyEvent(key) => {
                    if let Some(panel) = &mut self.state.side_panel {
                        if let Some(component) = panel.as_component() {
                            match component.handle_event(key) {
                                Ok(action) => self.handle_action(action, terminal)?,
                                Err(e) => {
                                    self.state.prompt = PromptMode::Error {
                                        message: format!("{e}"),
                                    };
                                }
                            }
                        }
                    }
                }
                AppEventResult::None => {}
            }

            // 外部シェル起動（既存フラグベース → 段階的に Action に移行予定）
            if self.state.launch_shell {
                self.state.launch_shell = false;
                self.handle_action(Action::LaunchShell, terminal)?;
            }

            // 非同期結果の受信
            self.state.receive_async_results();

            // カレントディレクトリの監視
            let current_dir_path = self.state.filer.current_dir.absolute_path();
            if current_dir_path != watching_dir_path {
                self.event_handler.watch_directory(current_dir_path)?;
                watching_dir_path = current_dir_path.to_string();
            }
        }
        Ok(())
    }
}
