use ratatui::DefaultTerminal;

use crate::cmd::prompt;
use crate::component::{Action, Component};
use crate::config::Config;
use crate::event::EventHandler;
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
        let dir = state.filer.state.current_dir.absolute_path();

        event_handler.pause();

        crossterm::terminal::disable_raw_mode()?;
        execute!(stdout(), LeaveAlternateScreen)?;

        let result = std::process::Command::new(&shell)
            .current_dir(dir)
            .status()
            .with_context(|| format!("シェルの起動に失敗しました: {shell}"));

        let r1 = execute!(stdout(), EnterAlternateScreen);
        let r2 = crossterm::terminal::enable_raw_mode();
        let r3 = terminal.clear();

        event_handler.resume();

        r1.and(r2).and(r3)?;

        result?;
        Ok(())
    }

    fn set_error(&mut self, message: String) {
        self.state.prompt.mode = PromptMode::Error { message };
    }

    /// Action を処理する
    fn handle_action(&mut self, action: Action, terminal: &mut DefaultTerminal) -> Result<()> {
        match action {
            Action::None => {}
            Action::Quit => self.state.quit(),
            Action::Error(message) => {
                self.set_error(message);
            }
            Action::LaunchShell => {
                if let Err(e) =
                    Self::launch_external_shell(&self.state, terminal, &self.event_handler)
                {
                    self.set_error(format!("{e}"));
                }
            }
            Action::CloseSidePanel => {
                self.state.side_panel = None;
            }
            Action::NavigateTo(path) => {
                self.state.side_panel = None;
                self.state.filer.state.jump_to(&path)?;
            }
            Action::RemoveBookmark(path) => {
                self.store.bookmark.remove(&path)?;
            }
            Action::AddBookmark(path) => {
                self.store.bookmark.add(&path)?;
            }
            Action::ExecutePrompt(input) => {
                prompt::execute_prompt_action(&mut self.state, &mut self.store, *input)?;
            }
            Action::CancelPrompt => {
                if let PromptMode::Search { original_index, .. } = &self.state.prompt.mode {
                    self.state.filer.state.file_table_state.select(*original_index);
                }
                self.state.prompt.mode = PromptMode::None;
            }
            Action::SearchUpdate(value) => {
                self.state.filer.state.select_matching_file(&value);
            }
            Action::SearchNext(value) => {
                self.state.filer.state.select_next_matching_file(&value);
            }
            Action::SearchPrev(value) => {
                self.state.filer.state.select_prev_matching_file(&value);
            }
            Action::SetPromptMode(mode) => {
                self.state.prompt.mode = *mode;
            }
            Action::ShowSidePanel(panel) => {
                if self.state.side_panel.is_none() {
                    self.state.side_panel = Some(panel);
                }
            }
            Action::OpenFile(path) => {
                open::that(path)?;
            }
            Action::RefreshFiles => {
                self.state.filer.state.refresh_files()?;
            }
            Action::KeyEvent(_) => {
                // KeyEvent は run() のメインループ内で直接処理されるため、ここには到達しない
            }
        }
        Ok(())
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let mut watching_dir_path =
            self.state.filer.state.current_dir.absolute_path().to_string();

        while self.state.running {
            // UI を描画
            terminal.draw(|frame| ui::render_main_view(frame, &mut self.state, &self.store))?;

            // イベントを取得して処理
            let event_action = self.event_handler.next_event()?;
            match event_action {
                Action::KeyEvent(key) => {
                    let action = if self.state.prompt.mode.is_active() {
                        self.state.prompt.handle_event(key)?
                    } else if let Some(panel) = self.state.side_panel.as_mut() {
                        panel.handle_event(key)?
                    } else {
                        // Filer: bookmark 表示は store が必要なため特別処理
                        if key.code == crossterm::event::KeyCode::Char('b') {
                            let paths = self.store.bookmark.get_paths().cloned().collect();
                            Action::ShowSidePanel(crate::state::SidePanel::Bookmark(
                                crate::component::BookmarkComponent::new(paths),
                            ))
                        } else {
                            self.state.filer.handle_event(key)?
                        }
                    };
                    if let Err(e) = self.handle_action(action, terminal) {
                        self.set_error(format!("{e}"));
                    }
                }
                Action::None => {}
                action => {
                    if let Err(e) = self.handle_action(action, terminal) {
                        self.set_error(format!("{e}"));
                    }
                }
            }

            // コンポーネントのtick処理（非同期結果の受信等）
            self.state.tick();

            // Filer のアクティブ状態を更新
            self.state
                .filer
                .set_active(self.state.side_panel.is_none() && !self.state.prompt.mode.is_active());

            // カレントディレクトリの監視
            let current_dir_path = self.state.filer.state.current_dir.absolute_path();
            if current_dir_path != watching_dir_path {
                self.event_handler.watch_directory(current_dir_path)?;
                watching_dir_path = current_dir_path.to_string();
            }
        }
        Ok(())
    }
}
