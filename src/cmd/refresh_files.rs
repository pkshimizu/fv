use crate::state::AppState;
use anyhow::Result;

pub fn exec(state: &mut AppState) -> Result<()> {
    state.filer.refresh_files()
}
