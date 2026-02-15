use crate::state::AppState;

pub fn exec(state: &mut AppState) {
    state.running = false;
}
