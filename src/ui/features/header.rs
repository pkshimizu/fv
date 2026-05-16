use crate::app_context::AppContext;
use crate::ui::widgets::{BorderStyle, build_bordered_block};
use ratatui::widgets::Widget;

pub fn build_header(ctx: &AppContext) -> impl Widget {
    build_bordered_block(
        format!("{}<0.0.0>", ctx.config.app_name).as_str(),
        BorderStyle::Inactive,
    )
}
