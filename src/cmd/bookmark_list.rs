use crate::state::{AppState, BookmarkListState};
use anyhow::{Context, Result};

pub fn show(state: &mut AppState) -> Result<()> {
    state.bookmark_list = Some(BookmarkListState::new(&state.filer.bookmarked_paths));
    Ok(())
}

pub fn close(state: &mut AppState) -> Result<()> {
    state.bookmark_list = None;
    Ok(())
}

pub fn up_cursor(state: &mut AppState) -> Result<()> {
    if let Some(bookmark_list) = &mut state.bookmark_list {
        bookmark_list.prev();
    }
    Ok(())
}

pub fn down_cursor(state: &mut AppState) -> Result<()> {
    if let Some(bookmark_list) = &mut state.bookmark_list {
        bookmark_list.next();
    }
    Ok(())
}

pub fn select(state: &mut AppState) -> Result<()> {
    let selected = state
        .bookmark_list
        .as_ref()
        .and_then(|bl| bl.selected_path().map(String::from));
    state.bookmark_list = None;

    if let Some(path) = selected {
        state
            .filer
            .navigate_to(&path)
            .context("Failed to navigate to bookmark")?;
    }
    Ok(())
}
