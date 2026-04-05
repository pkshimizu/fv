use crate::state::AppState;

pub fn exec(state: &mut AppState) -> anyhow::Result<()> {
    let file = state.filer.selected_file();
    if let Some(file) = file {
        if file.metadata()?.is_dir() {
            state.filer.change_dir_in_select_dir()?
        } else {
            open::that(file.absolute_path())?
        }
    }
    Ok(())
}
