use crate::state::AppState;
use crate::store::RootStore;
use anyhow::{Context, Result};

pub fn up_cursor(state: &mut AppState) -> Result<()> {
    if let Some(grep) = &mut state.grep {
        grep.prev();
    }
    Ok(())
}

pub fn down_cursor(state: &mut AppState) -> Result<()> {
    if let Some(grep) = &mut state.grep {
        grep.next();
    }
    Ok(())
}

pub fn first_cursor(state: &mut AppState) -> Result<()> {
    if let Some(grep) = &mut state.grep {
        grep.first();
    }
    Ok(())
}

pub fn last_cursor(state: &mut AppState) -> Result<()> {
    if let Some(grep) = &mut state.grep {
        grep.last();
    }
    Ok(())
}

pub fn select(state: &mut AppState) -> Result<()> {
    let selected = state
        .grep
        .as_ref()
        .and_then(|grep_state| grep_state.selected_path().map(String::from));
    state.grep = None;

    if let Some(path) = selected {
        state
            .filer
            .jump_to(&path)
            .context("Failed to navigate")?;
    }
    Ok(())
}

pub fn hide_grep(state: &mut AppState) -> Result<()> {
    state.grep = None;
    Ok(())
}
