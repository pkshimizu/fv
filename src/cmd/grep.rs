use crate::state::{AppState, SidePanel};
use anyhow::{Context, Result};

pub fn up_cursor(state: &mut AppState) -> Result<()> {
    if let Some(SidePanel::Grep(grep)) = &mut state.side_panel {
        grep.prev();
    }
    Ok(())
}

pub fn down_cursor(state: &mut AppState) -> Result<()> {
    if let Some(SidePanel::Grep(grep)) = &mut state.side_panel {
        grep.next();
    }
    Ok(())
}

pub fn first_cursor(state: &mut AppState) -> Result<()> {
    if let Some(SidePanel::Grep(grep)) = &mut state.side_panel {
        grep.first();
    }
    Ok(())
}

pub fn last_cursor(state: &mut AppState) -> Result<()> {
    if let Some(SidePanel::Grep(grep)) = &mut state.side_panel {
        grep.last();
    }
    Ok(())
}

pub fn select(state: &mut AppState) -> Result<()> {
    let selected = match &state.side_panel {
        Some(SidePanel::Grep(grep)) => grep.selected_path().map(String::from),
        _ => None,
    };
    state.side_panel = None;

    if let Some(path) = selected {
        state
            .filer
            .jump_to(&path)
            .context("Failed to navigate to grep result")?;
    }
    Ok(())
}

pub fn hide_grep(state: &mut AppState) -> Result<()> {
    state.side_panel = None;
    Ok(())
}
