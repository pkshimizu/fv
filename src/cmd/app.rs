use crate::state::AppState;
use anyhow::Result;

pub fn quit(state: &mut AppState) -> Result<()> {
    state.quit();
    Ok(())
}
