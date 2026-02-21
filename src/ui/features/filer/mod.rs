mod file_table;

use crate::ui::features::filer::file_table::build_file_table;
use ratatui::widgets::{Block, Table};
use std::fs;

pub fn build_filer(current_path: &str) -> Table {
    let files = fs::read_dir(current_path).unwrap().collect::<Vec<_>>();
    let block = Block::bordered().title(format!("{} ({})", current_path, files.len()));
    build_file_table(block, files)
}
