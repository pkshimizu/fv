use crate::cmd::{app, attribute, file_info, filer};
use crate::state::AppState;
use crate::store::RootStore;
use anyhow::Result;

pub enum Command {
    App(AppCommand),
    Filer(FilerCommand),
}

impl Command {
    pub fn exec(self, state: &mut AppState, store: &mut RootStore) -> Result<()> {
        match self {
            Command::App(cmd) => cmd.exec(state),
            Command::Filer(cmd) => cmd.exec(state, store),
        }
    }
}

pub enum AppCommand {
    Quit,
    None,
}

impl AppCommand {
    fn exec(self, state: &mut AppState) -> Result<()> {
        match self {
            AppCommand::Quit => app::quit(state),
            AppCommand::None => Ok(()),
        }
    }
}

pub enum FilerCommand {
    MoveCursorUp,
    MoveCursorDown,
    MoveCursorLeft,
    MoveCursorRight,
    EnterFile,
    ChangeParentDir,
    PromptCopy,
    PromptDelete,
    PromptMkdir,
    PromptTouch,
    PromptZip,
    PromptMove,
    PromptRename,
    PromptSort,
    PromptSearch,
    PromptGrep,
    PromptJump,
    AddBookmark,
    RemoveBookmark,
    ShowBookmark,
    RefreshFiles,
    ToggleCheckedFile,
    ShowAttribute,
    ShowFileInfo,
    ToggleDotFiles,
    LaunchShell,
}

impl FilerCommand {
    fn exec(self, state: &mut AppState, store: &mut RootStore) -> Result<()> {
        match self {
            FilerCommand::MoveCursorUp => filer::up_cursor(state),
            FilerCommand::MoveCursorDown => filer::down_cursor(state),
            FilerCommand::MoveCursorLeft => filer::first_cursor(state),
            FilerCommand::MoveCursorRight => filer::last_cursor(state),
            FilerCommand::EnterFile => filer::enter_file(state),
            FilerCommand::ChangeParentDir => filer::change_to_parent(state),
            FilerCommand::PromptCopy => filer::prompt_copy(state),
            FilerCommand::PromptDelete => filer::prompt_delete(state),
            FilerCommand::PromptMkdir => filer::prompt_mkdir(state),
            FilerCommand::PromptTouch => filer::prompt_touch(state),
            FilerCommand::PromptZip => filer::prompt_zip(state),
            FilerCommand::PromptMove => filer::prompt_move(state),
            FilerCommand::PromptRename => filer::prompt_rename(state),
            FilerCommand::PromptSort => filer::prompt_sort(state),
            FilerCommand::PromptSearch => filer::prompt_search(state),
            FilerCommand::PromptGrep => filer::prompt_grep(state),
            FilerCommand::LaunchShell => filer::launch_shell(state),
            FilerCommand::PromptJump => filer::prompt_jump(state),
            FilerCommand::AddBookmark => filer::add_bookmark(state, store),
            FilerCommand::RemoveBookmark => filer::remove_bookmark(state, store),
            FilerCommand::ShowBookmark => filer::show_bookmark(state, store),
            FilerCommand::RefreshFiles => filer::refresh_files(state),
            FilerCommand::ToggleCheckedFile => filer::toggle_checked_file(state),
            FilerCommand::ShowAttribute => attribute::show_attribute(state),
            FilerCommand::ShowFileInfo => file_info::show_file_info(state),
            FilerCommand::ToggleDotFiles => filer::toggle_dot_files(state),
        }
    }
}
