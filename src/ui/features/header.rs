use crate::state::AppContext;
use crate::ui::widgets::{BorderStyle, build_bordered_block};
use ratatui::widgets::Widget;

pub fn build_header(state: &AppContext) -> impl Widget {
    build_bordered_block(
        format!("{}<0.0.0>", state.config.app_name).as_str(),
        BorderStyle::Inactive,
    )
}
