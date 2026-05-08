use crate::state::{AppState, SidePanel};
use crate::store::RootStore;
use anyhow::{Context, Result};

pub fn up_cursor(state: &mut AppState) -> Result<()> {
    if let Some(SidePanel::Bookmark(bookmark)) = &mut state.side_panel {
        bookmark.prev();
    }
    Ok(())
}

pub fn down_cursor(state: &mut AppState) -> Result<()> {
    if let Some(SidePanel::Bookmark(bookmark)) = &mut state.side_panel {
        bookmark.next();
    }
    Ok(())
}

pub fn first_cursor(state: &mut AppState) -> Result<()> {
    if let Some(SidePanel::Bookmark(bookmark)) = &mut state.side_panel {
        bookmark.first();
    }
    Ok(())
}

pub fn last_cursor(state: &mut AppState) -> Result<()> {
    if let Some(SidePanel::Bookmark(bookmark)) = &mut state.side_panel {
        bookmark.last();
    }
    Ok(())
}

pub fn select(state: &mut AppState) -> Result<()> {
    if !matches!(state.side_panel, Some(SidePanel::Bookmark(_))) {
        return Ok(());
    }
    let Some(SidePanel::Bookmark(bookmark)) = state.side_panel.take() else {
        unreachable!()
    };
    if let Some(path) = bookmark.selected_path() {
        state
            .filer
            .jump_to(path)
            .context("Failed to navigate to bookmark")?;
    }
    Ok(())
}

pub fn remove_bookmark(state: &mut AppState, store: &mut RootStore) -> Result<()> {
    if let Some(SidePanel::Bookmark(bookmark)) = &mut state.side_panel {
        if let Some(path) = bookmark.selected_path().map(String::from) {
            store.bookmark.remove(&path)?;
            bookmark.remove(&path);
        }
    }
    Ok(())
}

pub fn hide_bookmark(state: &mut AppState) -> Result<()> {
    if matches!(state.side_panel, Some(SidePanel::Bookmark(_))) {
        state.side_panel = None;
    }
    Ok(())
}
