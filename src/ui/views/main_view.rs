use crate::component::Component;
use crate::app_context::AppContext;
use crate::store::RootStore;
use crate::ui::features::build_header;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};

pub fn render_main_view(frame: &mut Frame, ctx: &mut AppContext, store: &RootStore) {
    let area = frame.area();

    let [header_area, content_area, prompt_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(3),
    ])
    .areas(area);

    frame.render_widget(build_header(ctx), header_area);
    match &mut ctx.side_panel {
        Some(panel) => {
            let [filer_area, panel_area] =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(content_area);
            ctx.filer.render_with_store(frame, filer_area, store);
            panel.render(frame, panel_area);
        }
        None => {
            ctx.filer.render_with_store(frame, content_area, store);
        }
    }
    ctx.prompt.render(frame, prompt_area);
}
