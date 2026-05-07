use crate::state::{AppState, AttributeState};
use anyhow::Result;

pub fn show_attribute(state: &mut AppState) -> Result<()> {
    if let Some(file) = state.filer.selected_file() {
        state.attribute = Some(AttributeState::new(file)?);
    }
    Ok(())
}

pub fn hide_attribute(state: &mut AppState) -> Result<()> {
    state.attribute = None;
    Ok(())
}

pub fn up_cursor(state: &mut AppState) -> Result<()> {
    if let Some(attr) = &mut state.attribute {
        attr.prev();
    }
    Ok(())
}

pub fn down_cursor(state: &mut AppState) -> Result<()> {
    if let Some(attr) = &mut state.attribute {
        attr.next();
    }
    Ok(())
}
