use crate::component::FileInfoComponent;
use crate::state::{AppState, SidePanel};
use anyhow::Result;

pub fn show_file_info(state: &mut AppState) -> Result<()> {
    if state.side_panel.is_some() {
        return Ok(());
    }
    let Some(file) = state.filer.selected_file() else {
        return Ok(());
    };
    state.side_panel = Some(SidePanel::FileInfo(FileInfoComponent::new(file)?));
    Ok(())
}
