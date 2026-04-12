use crate::state::AppState;
use anyhow::Result;

pub fn change_to_parent(state: &mut AppState) -> Result<()> {
    state.filer.change_dir_in_parent_dir()
}

pub fn up_cursor(state: &mut AppState) -> Result<()> {
    Ok(state.filer.prev())
}

pub fn down_cursor(state: &mut AppState) -> Result<()> {
    Ok(state.filer.next())
}

pub fn first_cursor(state: &mut AppState) -> Result<()> {
    Ok(state.filer.first())
}

pub fn last_cursor(state: &mut AppState) -> Result<()> {
    Ok(state.filer.last())
}

pub fn refresh_files(state: &mut AppState) -> Result<()> {
    state.filer.refresh_files()
}

pub fn toggle_checked_file(state: &mut AppState) -> Result<()> {
    state.filer.toggle_checked_file();
    state.filer.next();
    Ok(())
}
