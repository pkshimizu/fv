use crate::state::{AppState, SidePanel};
use anyhow::Result;

pub fn scroll_up(state: &mut AppState) -> Result<()> {
    if let Some(SidePanel::Shell(shell)) = &mut state.side_panel {
        shell.scroll_up();
    }
    Ok(())
}

pub fn scroll_down(state: &mut AppState) -> Result<()> {
    if let Some(SidePanel::Shell(shell)) = &mut state.side_panel {
        shell.scroll_down();
    }
    Ok(())
}

pub fn scroll_to_top(state: &mut AppState) -> Result<()> {
    if let Some(SidePanel::Shell(shell)) = &mut state.side_panel {
        shell.scroll_to_top();
    }
    Ok(())
}

pub fn scroll_to_bottom(state: &mut AppState) -> Result<()> {
    if let Some(SidePanel::Shell(shell)) = &mut state.side_panel {
        shell.scroll_to_bottom();
    }
    Ok(())
}

pub fn hide_shell(state: &mut AppState) -> Result<()> {
    if matches!(state.side_panel, Some(SidePanel::Shell(_))) {
        state.side_panel = None;
    }
    Ok(())
}
