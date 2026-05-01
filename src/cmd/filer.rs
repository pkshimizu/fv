use crate::bookmark;
use crate::state::AppState;
use anyhow::{Context, Result};

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

pub fn add_bookmark(state: &mut AppState) -> Result<()> {
    if let Some(path) = state.filer.selected_bookmark_path() {
        state.filer.insert_bookmark(path.clone());
        if let Err(e) = bookmark::save_bookmarks(&state.filer.bookmarked_paths)
            .context("Failed to add bookmark")
        {
            state.filer.remove_bookmark(&path);
            return Err(e);
        }
    }
    Ok(())
}

pub fn remove_bookmark(state: &mut AppState) -> Result<()> {
    if let Some(path) = state.filer.selected_unbookmark_path() {
        state.filer.remove_bookmark(&path);
        if let Err(e) = bookmark::save_bookmarks(&state.filer.bookmarked_paths)
            .context("Failed to remove bookmark")
        {
            state.filer.insert_bookmark(path);
            return Err(e);
        }
    }
    Ok(())
}
