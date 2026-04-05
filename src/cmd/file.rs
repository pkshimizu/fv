use crate::state::AppState;

pub fn enter_file(state: &mut AppState) -> anyhow::Result<()> {
    let Some(file) = state.filer.selected_file() else {
        return Ok(());
    };
    let is_dir = file.is_dir()?;
    let path = file.absolute_path().to_string();
    if is_dir {
        state.filer.change_to(&path)?;
    } else {
        open::that(path)?;
    }
    Ok(())
}
