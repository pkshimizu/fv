use crate::state::{AppState, BookmarkListState};
use anyhow::Result;

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
