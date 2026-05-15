use crate::component::AttributeComponent;
use crate::state::{AppState, SidePanel};
use anyhow::Result;

pub fn show_attribute(state: &mut AppState) -> Result<()> {
    if state.side_panel.is_some() {
        return Ok(());
    }
    let Some(file) = state.filer.selected_file() else {
        return Ok(());
    };
    state.side_panel = Some(SidePanel::Attribute(AttributeComponent::new(file)?));
    Ok(())
}
