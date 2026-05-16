use crate::component::Component;
use crate::state::AppState;
use crate::store::RootStore;
use crate::ui::features::build_header;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};

pub fn render_main_view(frame: &mut Frame, state: &mut AppState, store: &RootStore) {
    let area = frame.area();

    let [header_area, content_area, prompt_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(3),
    ])
    .areas(area);

    frame.render_widget(build_header(state), header_area);
    match &mut state.side_panel {
        Some(panel) => {
            let [filer_area, panel_area] =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(content_area);
            state.filer.render_with_store(frame, filer_area, store);
            panel.render(frame, panel_area);
        }
        None => {
            state.filer.render_with_store(frame, content_area, store);
        }
    }
    state.prompt.render(frame, prompt_area);
}
