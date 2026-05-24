use ratatui::DefaultTerminal;

use crate::app_context::AppContext;
use crate::component::{Action, Component, prompt};
use crate::config::Config;
use crate::event::{EventHandler, InputEvent};
use crate::store::RootStore;
use crate::ui;
use anyhow::{Context, Result};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use std::io::stdout;

pub struct App {
    ctx: AppContext,
    store: RootStore,
    event_handler: EventHandler,
}

impl App {
    pub fn new(config: Config, picker: ratatui_image::picker::Picker) -> Result<Self> {
        Ok(Self {
            ctx: AppContext::new(config, picker),
            store: RootStore::new()?,
            event_handler: EventHandler::default(),
        })
    }

    pub fn init(&mut self) -> Result<()> {
        if let Err(e) = self.store.init() {
            tracing::warn!("Failed to initialize store: {}", e);
        }
        let startup_dir = self.resolve_startup_directory();
        self.ctx.init(startup_dir)?;
        Ok(())
    }

    fn resolve_startup_directory(&self) -> Option<std::path::PathBuf> {
        use crate::store::StartupDirectory;
        match self.store.settings.startup_directory() {
            StartupDirectory::CurrentDirectory => None,
            StartupDirectory::HomeDirectory => dirs::home_dir(),
        }
    }

    fn launch_external_shell(
        ctx: &AppContext,
        terminal: &mut DefaultTerminal,
        event_handler: &EventHandler,
    ) -> Result<()> {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let dir = ctx.filer.current_dir_path();

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
        self.ctx.prompt.set_error(message);
    }

    /// Action を処理する
    fn handle_action(&mut self, action: Action, terminal: &mut DefaultTerminal) -> Result<()> {
        match action {
            Action::None => {}
            Action::Quit => self.ctx.quit(),
            Action::Error(message) => {
                self.set_error(message);
            }
            Action::LaunchShell => {
                if let Err(e) =
                    Self::launch_external_shell(&self.ctx, terminal, &self.event_handler)
                {
                    self.set_error(format!("{e}"));
                }
            }
            Action::CloseSidePanel => {
                self.ctx.side_panel = None;
            }
            Action::NavigateTo(path) => {
                self.ctx.side_panel = None;
                self.ctx.filer.jump_to(&path)?;
            }
            Action::RemoveBookmark(path) => {
                self.store.bookmark.remove(&path)?;
            }
            Action::AddBookmark(path) => {
                self.store.bookmark.add(&path)?;
            }
            Action::ExecutePrompt(input) => {
                prompt::execute_prompt_action(&mut self.ctx, &mut self.store, *input)?;
            }
            Action::CancelPrompt => {
                if let Some(idx) = self.ctx.prompt.cancel() {
                    self.ctx.filer.select_file_table(Some(idx));
                }
            }
            Action::SearchUpdate(value) => {
                self.ctx.filer.select_matching_file(&value);
            }
            Action::SearchNext(value) => {
                self.ctx.filer.select_next_matching_file(&value);
            }
            Action::SearchPrev(value) => {
                self.ctx.filer.select_prev_matching_file(&value);
            }
            Action::SetPromptMode(mode) => {
                self.ctx.prompt.set_mode(*mode);
            }
            Action::ShowSidePanel(panel) => {
                if self.ctx.side_panel.is_none() {
                    self.ctx.side_panel = Some(panel);
                }
            }
            Action::OpenFile(path) => {
                open::that(path)?;
            }
            Action::ShowBookmark => {
                if self.ctx.side_panel.is_none() {
                    let paths = self.store.bookmark.get_paths().cloned().collect();
                    self.ctx.side_panel = Some(crate::state::SidePanel::Bookmark(
                        crate::component::BookmarkComponent::new(paths),
                    ));
                }
            }
            Action::ShowSettings => {
                if self.ctx.side_panel.is_none() {
                    let startup_dir = self.store.settings.startup_directory().clone();
                    self.ctx.side_panel = Some(crate::state::SidePanel::Settings(
                        crate::component::SettingsComponent::new(&startup_dir),
                    ));
                }
            }
            Action::SaveSettings(startup_dir) => {
                self.store.settings.set_startup_directory(*startup_dir)?;
                self.ctx.side_panel = None;
            }
        }
        Ok(())
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let mut watching_dir_path = self.ctx.filer.current_dir_path().to_string();

        while self.ctx.running {
            // UI を描画
            terminal.draw(|frame| ui::render_main_view(frame, &mut self.ctx, &self.store))?;

            // イベントを取得して処理
            match self.event_handler.next_event()? {
                InputEvent::Key(key) => {
                    let action = if self.ctx.prompt.is_active() {
                        self.ctx.prompt.handle_event(key)?
                    } else if let Some(panel) = self.ctx.side_panel.as_mut() {
                        panel.handle_event(key)?
                    } else {
                        self.ctx.filer.handle_event(key)?
                    };
                    if let Err(e) = self.handle_action(action, terminal) {
                        self.set_error(format!("{e}"));
                    }
                }
                InputEvent::FileChange => {
                    if !self.ctx.filer.is_loading() {
                        self.ctx.filer.refresh_files();
                    }
                }
                InputEvent::None => {}
            }

            // コンポーネントのtick処理（非同期結果の受信等）
            self.ctx.tick();

            // 非同期ロードのエラーを検知して表示
            if let Some(error) = self.ctx.filer.take_error() {
                self.set_error(error);
            }

            // Filer のアクティブ状態を更新
            self.ctx
                .filer
                .set_active(self.ctx.side_panel.is_none() && !self.ctx.prompt.is_active());

            // カレントディレクトリの監視
            let current_dir_path = self.ctx.filer.current_dir_path();
            if current_dir_path != watching_dir_path {
                self.event_handler.watch_directory(current_dir_path)?;
                watching_dir_path = current_dir_path.to_string();
            }
        }
        Ok(())
    }
}
