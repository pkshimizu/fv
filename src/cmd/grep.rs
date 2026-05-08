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
    let Some(SidePanel::Grep(grep)) = &state.side_panel.take() else {
        return Ok(());
    };
    if let Some(path) = grep.selected_path() {
        state.filer.jump_to(path).context("Failed to navigate to grep result")?;
    }
    state.side_panel = None;
    Ok(())
}

pub fn hide_grep(state: &mut AppState) -> Result<()> {
    if matches!(state.side_panel, Some(SidePanel::Grep(_))) {
        state.side_panel = None;
    }
    Ok(())
}
