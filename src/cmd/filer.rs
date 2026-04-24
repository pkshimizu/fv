use crate::state::AppState;
use anyhow::Result;

pub fn change_to_parent(state: &mut AppState) -> Result<()> {
    state.filer.change_dir_in_parent_dir()
}

pub fn up_cursor(state: &mut AppState) -> Result<()> {
    state.filer.prev();
    Ok(())
}

pub fn down_cursor(state: &mut AppState) -> Result<()> {
    state.filer.next();
    Ok(())
}

pub fn first_cursor(state: &mut AppState) -> Result<()> {
    state.filer.first();
    Ok(())
}

pub fn last_cursor(state: &mut AppState) -> Result<()> {
    state.filer.last();
    Ok(())
}

pub fn refresh_files(state: &mut AppState) -> Result<()> {
    state.filer.refresh_files()
}

pub fn toggle_checked_file(state: &mut AppState) -> Result<()> {
    state.filer.toggle_checked_file();
    state.filer.next();
    Ok(())
}

pub fn toggle_dot_files(state: &mut AppState) -> Result<()> {
    state.filer.toggle_show_dot_file()
}
