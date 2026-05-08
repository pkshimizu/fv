use crate::state::{AppState, AttributeState, SidePanel};
use anyhow::Result;

pub fn show_attribute(state: &mut AppState) -> Result<()> {
    if let Some(file) = state.filer.selected_file() {
        state.side_panel = Some(SidePanel::Attribute(AttributeState::new(file)?));
    }
    Ok(())
}

pub fn hide_attribute(state: &mut AppState) -> Result<()> {
    if matches!(state.side_panel, Some(SidePanel::Attribute(_))) {
        state.side_panel = None;
    }
    Ok(())
}

pub fn up_cursor(state: &mut AppState) -> Result<()> {
    if let Some(SidePanel::Attribute(attr)) = &mut state.side_panel {
        attr.prev();
    }
    Ok(())
}

pub fn down_cursor(state: &mut AppState) -> Result<()> {
    if let Some(SidePanel::Attribute(attr)) = &mut state.side_panel {
        attr.next();
    }
    Ok(())
}
