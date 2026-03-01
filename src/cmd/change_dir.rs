use crate::state::AppState;

pub fn exec(state: &mut AppState) {
    state.filer.change_dir_in_select_file()
}
