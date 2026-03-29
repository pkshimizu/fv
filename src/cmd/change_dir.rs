use crate::state::AppState;
use anyhow::Result;

pub fn select_dir(state: &mut AppState) -> Result<()> {
    state.filer.change_dir_in_select_dir()
}

pub fn parent_dir(state: &mut AppState) -> Result<()> {
    state.filer.change_dir_in_parent_dir()
}
