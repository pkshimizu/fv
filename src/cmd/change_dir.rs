use crate::state::AppState;
use anyhow::Result;

pub fn parent_dir(state: &mut AppState) -> Result<()> {
    state.filer.change_dir_in_parent_dir()
}
