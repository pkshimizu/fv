mod file_table;

use crate::state::FilerState;
use crate::ui::features::filer::file_table::build_file_table;
use ratatui::widgets::{Block, Table};

pub fn build_filer(state: &FilerState) -> Table<'static> {
    let list_size = state.current_dir.list_size();
    let block = Block::bordered().title(format!(
        "{} ({})",
        state.current_dir.absolute_path(),
        list_size
            .map(|n| n.to_string())
            .unwrap_or_else(|_| "?".to_string())
    ));
    build_file_table(block, &state.current_dir_files)
}
