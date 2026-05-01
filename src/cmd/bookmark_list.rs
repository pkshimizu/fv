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

pub fn select(state: &mut AppState) -> Result<()> {
    if let Some(bookmark_list) = &state.bookmark_list {
        if let Some(path) = bookmark_list.selected_path() {
            let path = std::path::Path::new(path);
            if let Some(parent) = path.parent() {
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(String::from);
                state
                    .filer
                    .change_to(parent.to_str().unwrap_or_default())?;
                if let Some(name) = file_name {
                    state.filer.set_pending_select_name(name);
                    state.filer.refresh_files()?;
                }
            }
        }
    }
    state.bookmark_list = None;
    Ok(())
}
