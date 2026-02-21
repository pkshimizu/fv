mod file_table;

use crate::state::AppState;
use crate::ui::features::filer::file_table::build_file_table;
use ratatui::prelude::Widget;
use ratatui::widgets::Block;
use std::fs;

pub fn build_filer(state: &AppState) -> impl Widget {
    let current_path = state.filer.current_dir_path.to_str().unwrap();
    let files = fs::read_dir(current_path).unwrap().collect::<Vec<_>>();
    let block = Block::bordered().title(format!(
        "{} ({})",
        state.filer.current_dir_path.to_str().unwrap(),
        files.len()
    ));
    build_file_table(block, files)
}
