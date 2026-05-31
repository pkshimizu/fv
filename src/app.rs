pub mod async_job;

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
    skip_history_add: bool,
}

impl App {
    pub fn new(config: Config, picker: ratatui_image::picker::Picker) -> Result<Self> {
        Ok(Self {
            ctx: AppContext::new(config, picker),
            store: RootStore::new()?,
            event_handler: EventHandler::default(),
            skip_history_add: false,
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
            StartupDirectory::LastDirectory => self
                .store
                .history
                .last_entry()
                .filter(|p| std::path::Path::new(p).is_dir())
                .map(std::path::PathBuf::from)
                .or_else(dirs::home_dir),
        }
    }

    /// alternate screen を離脱してクロージャを実行し、完了後に復帰する。
    fn run_in_shell_mode<F>(
        terminal: &mut DefaultTerminal,
        event_handler: &EventHandler,
        f: F,
    ) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        event_handler.pause();

        crossterm::terminal::disable_raw_mode()?;
        execute!(stdout(), LeaveAlternateScreen)?;

        let result = f();

        let r1 = execute!(stdout(), EnterAlternateScreen);
        let r2 = crossterm::terminal::enable_raw_mode();
        let r3 = terminal.clear();

        event_handler.resume();

        r1.and(r2).and(r3)?;

        result
    }

    fn default_shell() -> String {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
    }

    fn launch_external_shell(
        ctx: &AppContext,
        terminal: &mut DefaultTerminal,
        event_handler: &EventHandler,
    ) -> Result<()> {
        let shell = Self::default_shell();
        let dir = ctx.filer.current_dir_path().to_string();

        Self::run_in_shell_mode(terminal, event_handler, || {
            std::process::Command::new(&shell)
                .current_dir(&dir)
                .status()
                .with_context(|| format!("シェルの起動に失敗しました: {shell}"))?;
            Ok(())
        })
    }

    fn execute_shell_command(
        command: String,
        dir: String,
        terminal: &mut DefaultTerminal,
        event_handler: &EventHandler,
    ) -> Result<()> {
        let shell = Self::default_shell();

        Self::run_in_shell_mode(terminal, event_handler, || {
            std::process::Command::new(&shell)
                .arg("-c")
                .arg(&command)
                .current_dir(&dir)
                .status()
                .with_context(|| format!("コマンドの実行に失敗しました: {command}"))?;

            eprintln!("\nPress Enter to continue...");
            let _ = std::io::stdin().read_line(&mut String::new());
            Ok(())
        })
    }

    fn set_error(&mut self, message: String) {
        self.ctx.prompt.set_error(message);
    }

    /// Action を処理する
    fn handle_action(&mut self, action: Action, terminal: &mut DefaultTerminal) -> Result<()> {
        match action {
            Action::None => {}
            Action::Quit => self.ctx.quit(),
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
            Action::ExecuteCommand(command, dir) => {
                if let Err(e) =
                    Self::execute_shell_command(command, dir, terminal, &self.event_handler)
                {
                    self.set_error(format!("{e}"));
                }
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
                    self.ctx.side_panel = Some(*panel);
                }
            }
            Action::OpenFile(path) => {
                open::that(path)?;
            }
            Action::Yank(paths) => {
                if let Err(e) = crate::os::clipboard::write_paths(&paths) {
                    tracing::warn!("yank failed: {e:#}");
                    self.set_error(format!("{e:#}"));
                }
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
            Action::NavigateBack => {
                if let Some(path) = self.store.history.back().map(String::from) {
                    self.ctx.filer.change_to(&path);
                    self.skip_history_add = true;
                }
            }
            Action::NavigateForward => {
                if let Some(path) = self.store.history.forward().map(String::from) {
                    self.ctx.filer.change_to(&path);
                    self.skip_history_add = true;
                }
            }
        }
        Ok(())
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let mut watching_dir_path = self.ctx.filer.current_dir_path().to_string();
        // 起動直後の初期ディレクトリにも監視を張る。ループ内の watch_directory は
        // ナビゲートで current_dir が変わったときしか呼ばれないため、ここで一度設定する。
        // 監視は自動更新のための付加機能なので、失敗してもアプリ起動は止めず警告に留める。
        if let Err(e) = self.event_handler.watch_directory(&watching_dir_path) {
            tracing::warn!("Failed to watch startup directory: {e}");
        }

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

            // カレントディレクトリの監視と履歴保存
            let current_dir_path = self.ctx.filer.current_dir_path();
            if current_dir_path != watching_dir_path {
                // 監視失敗は致命ではないため警告に留める（起動時と同方針）。
                if let Err(e) = self.event_handler.watch_directory(current_dir_path) {
                    tracing::warn!("Failed to watch directory: {e}");
                }
                if self.skip_history_add {
                    self.skip_history_add = false;
                } else if let Err(e) = self.store.history.add(current_dir_path) {
                    tracing::warn!("Failed to save history: {e}");
                }
                watching_dir_path = current_dir_path.to_string();
            }
        }
        Ok(())
    }
}
