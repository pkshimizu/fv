use crate::state::AppState;
use ratatui::widgets::Block;
use std::fs;

pub fn build_filer(state: &AppState) -> Block {
    let current_path = state.filer.current_dir_path.to_str().unwrap();
    let files = fs::read_dir(current_path).unwrap();
    Block::bordered().title(format!(
        "{} ({})",
        state.filer.current_dir_path.to_str().unwrap(),
        files.count()
    ))
}
