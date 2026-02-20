use crate::state::AppState;
use ratatui::widgets::Block;

pub fn build_header(state: &AppState) -> Block {
    Block::bordered().title(format!("{}<0.0.0>", state.config.app_name))
}
