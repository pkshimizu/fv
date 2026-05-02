mod file_table;

use crate::state::FilerState;
use crate::store::RootStore;
use crate::ui::features::filer::file_table::build_file_table;
use ratatui::widgets::{Block, Table};

pub fn build_filer(state: &FilerState, store: &RootStore) -> Table<'static> {
    let list_size = state.current_dir_files.len();
    let block = Block::bordered().title(format!(
        "{} ({})",
        state.current_dir.absolute_path(),
        list_size
    ));
    build_file_table(block, state, store)
}
