use crate::state::AppState;

pub fn up(state: &mut AppState) {
    state.filer.prev()
}

pub fn down(state: &mut AppState) {
    state.filer.next()
}

pub fn first(state: &mut AppState) {
    state.filer.first()
}

pub fn last(state: &mut AppState) {
    state.filer.last()
}
