mod file_table;

use crate::state::{AppState, Area};
use crate::store::RootStore;
use crate::ui::features::filer::file_table::build_file_table;
use crate::ui::widgets::build_bordered_block;
use ratatui::widgets::Table;

pub fn build_filer(state: &AppState, store: &RootStore) -> Table<'static> {
    let list_size = state.filer.current_dir_files.len();
    let title = format!(
        "{} ({})",
        state.filer.current_dir.absolute_path(),
        list_size
    );
    let block = build_bordered_block(title.as_str(), state.is_active(Area::Filer));
    build_file_table(block, &state.filer, store)
}
