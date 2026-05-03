use crate::state::AppState;
use crate::store::RootStore;
use anyhow::{Context, Result};

pub fn up_cursor(state: &mut AppState) -> Result<()> {
    if let Some(bookmark) = &mut state.bookmark {
        bookmark.prev();
    }
    Ok(())
}

pub fn down_cursor(state: &mut AppState) -> Result<()> {
    if let Some(bookmark) = &mut state.bookmark {
        bookmark.next();
    }
    Ok(())
}

pub fn first_cursor(state: &mut AppState) -> Result<()> {
    if let Some(bookmark) = &mut state.bookmark {
        bookmark.first();
    }
    Ok(())
}

pub fn last_cursor(state: &mut AppState) -> Result<()> {
    if let Some(bookmark) = &mut state.bookmark {
        bookmark.last();
    }
    Ok(())
}

pub fn select(state: &mut AppState) -> Result<()> {
    let selected = state
        .bookmark
        .as_ref()
        .and_then(|bookmark_state| bookmark_state.selected_path().map(String::from));
    state.bookmark = None;

    if let Some(path) = selected {
        state
            .filer
            .jump_to(&path)
            .context("Failed to navigate to bookmark")?;
    }
    Ok(())
}

pub fn remove_bookmark(state: &mut AppState, store: &mut RootStore) -> Result<()> {
    if let Some(bookmark) = &mut state.bookmark {
        if let Some(path) = bookmark.selected_path().map(String::from) {
            store.bookmark.remove(&path)?;
            bookmark.remove(&path);
        }
    }
    Ok(())
}

pub fn hide_bookmark(state: &mut AppState) -> Result<()> {
    state.bookmark = None;
    Ok(())
}
