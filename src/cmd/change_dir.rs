use crate::state::AppState;

pub fn select_dir(state: &mut AppState) {
    state.filer.change_dir_in_select_dir()
}

pub fn parent_dir(state: &mut AppState) {
    state.filer.change_dir_in_parent_dir()
}
