use crate::component::Component;
use crate::state::AppState;
use crate::store::RootStore;
use crate::ui::features::{build_filer, build_header};
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
    let filer = build_filer(state, store);
    match &mut state.side_panel {
        Some(panel) => {
            let [filer_area, panel_area] =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(content_area);
            frame.render_stateful_widget(filer, filer_area, &mut state.filer.file_table_state);
            panel.render(frame, panel_area);
        }
        None => {
            frame.render_stateful_widget(filer, content_area, &mut state.filer.file_table_state);
        }
    }
    state.prompt.render(frame, prompt_area);
}
