use crate::state::AppState;

pub fn quit(state: &mut AppState) -> anyhow::Result<()> {
    Ok(state.quit())
}
