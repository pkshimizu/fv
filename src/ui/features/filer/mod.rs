mod file_table;

use crate::state::FilerState;
use crate::ui::features::filer::file_table::build_file_table;
use ratatui::widgets::{Block, Table};

pub fn build_filer(state: &FilerState) -> Table<'static> {
    let block = Block::bordered().title(format!(
        "{} ({})",
        state.current_dir.absolute_path(),
        state.current_dir.list_size()
    ));
    build_file_table(block, &state.current_dir_files)
}
