use crate::state::AppState;
use crate::store::RootStore;
use anyhow::Result;

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
    Ok(())
}

pub fn remove_bookmark(state: &AppState, store: &mut RootStore) -> Result<()> {
    if let Some(selected_file) = state.filer.selected_file() {
        store.bookmark.remove(selected_file.absolute_path())?;
    }
    Ok(())
}

pub fn hide_bookmark(state: &mut AppState) -> Result<()> {
    state.bookmark = None;
    Ok(())
}
