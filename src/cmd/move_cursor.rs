use crate::state::AppState;
use anyhow::Result;

pub fn up(state: &mut AppState) -> Result<()> {
    Ok(state.filer.prev())
}

pub fn down(state: &mut AppState) -> Result<()> {
    Ok(state.filer.next())
}

pub fn first(state: &mut AppState) -> Result<()> {
    Ok(state.filer.first())
}

pub fn last(state: &mut AppState) -> Result<()> {
    Ok(state.filer.last())
}
