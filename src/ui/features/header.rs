use crate::state::AppState;
use ratatui::widgets::{Block, Widget};

pub fn build_header(state: &AppState) -> impl Widget {
    Block::bordered().title(format!("{}<0.0.0>", state.config.app_name))
}
