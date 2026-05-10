use crate::fs::file_info::FileInfo;
use crate::state::{AppState, SidePanel, TextOutputState};
use anyhow::Result;

pub fn show_file_info(state: &mut AppState) -> Result<()> {
    if state.side_panel.is_some() {
        return Ok(());
    }
    let Some(file) = state.filer.selected_file() else {
        return Ok(());
    };
    let info = FileInfo::from_file(file)?;
    let lines = info.to_lines();
    let mut output = TextOutputState::new(None);
    output.lines = lines;
    state.side_panel = Some(SidePanel::FileInfo(output));
    Ok(())
}

pub fn scroll_up(state: &mut AppState) -> Result<()> {
    if let Some(SidePanel::FileInfo(panel)) = &mut state.side_panel {
        panel.scroll_up();
    }
    Ok(())
}

pub fn scroll_down(state: &mut AppState) -> Result<()> {
    if let Some(SidePanel::FileInfo(panel)) = &mut state.side_panel {
        panel.scroll_down();
    }
    Ok(())
}

pub fn scroll_to_top(state: &mut AppState) -> Result<()> {
    if let Some(SidePanel::FileInfo(panel)) = &mut state.side_panel {
        panel.scroll_to_top();
    }
    Ok(())
}

pub fn scroll_to_bottom(state: &mut AppState) -> Result<()> {
    if let Some(SidePanel::FileInfo(panel)) = &mut state.side_panel {
        panel.scroll_to_bottom();
    }
    Ok(())
}

pub fn hide_file_info(state: &mut AppState) -> Result<()> {
    if matches!(state.side_panel, Some(SidePanel::FileInfo(_))) {
        state.side_panel = None;
    }
    Ok(())
}
