use crate::state::AppState;
use crate::store::RootStore;
use crate::ui::features::{build_filer, build_header, build_prompt_view};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};

pub fn render_main_view(frame: &mut Frame, state: &mut AppState, store: &RootStore) {
    let area = frame.area();

    let [header_area, filer_area, input_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(3),
    ])
    .areas(area);

    frame.render_widget(build_header(state), header_area);
    frame.render_stateful_widget(
        build_filer(&state.filer, store),
        filer_area,
        &mut state.filer.file_table_state,
    );
    frame.render_widget(build_prompt_view(&state.prompt), input_area);
}
