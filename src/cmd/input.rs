use crate::state::{AppState, InputMode};
use anyhow::Result;

pub fn input_char(state: &mut AppState, c: char) -> Result<()> {
    if let InputMode::Text { value, .. } = &mut state.input {
        value.push(c);
    }
    Ok(())
}

pub fn input_backspace(state: &mut AppState) -> Result<()> {
    if let InputMode::Text { value, .. } = &mut state.input {
        value.pop();
    }
    Ok(())
}

pub fn input_ok(state: &mut AppState) -> Result<()> {
    state.input = InputMode::None;
    Ok(())
}

pub fn input_cancel(state: &mut AppState) -> Result<()> {
    state.input = InputMode::None;
    Ok(())
}
