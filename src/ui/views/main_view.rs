use crate::state::AppState;
use crate::store::RootStore;
use crate::ui::features::bookmark::table::build_bookmark_table;
use crate::ui::features::{build_filer, build_header, build_prompt_view};
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
    if let Some(bookmark) = &mut state.bookmark {
        let [filer_area, bookmark_area] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(content_area);
        frame.render_stateful_widget(filer, filer_area, &mut state.filer.file_table_state);
        frame.render_stateful_widget(
            build_bookmark_table(bookmark),
            bookmark_area,
            &mut bookmark.table_state,
        )
    } else {
        frame.render_stateful_widget(filer, content_area, &mut state.filer.file_table_state);
    }
    frame.render_widget(build_prompt_view(&state.prompt), prompt_area);
}
