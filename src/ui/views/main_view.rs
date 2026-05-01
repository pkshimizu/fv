use crate::state::AppState;
use crate::ui::features::{build_bookmark_list, build_filer, build_header, build_input_area};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};

pub fn render_main_view(frame: &mut Frame, state: &mut AppState) {
    let area = frame.area();

    let [header_area, content_area, input_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(3),
    ])
    .areas(area);

    frame.render_widget(build_header(state), header_area);

    if let Some(bookmark_list) = &mut state.bookmark_list {
        let [filer_area, bookmark_area] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(content_area);

        frame.render_stateful_widget(
            build_filer(&state.filer),
            filer_area,
            &mut state.filer.file_table_state,
        );
        frame.render_stateful_widget(
            build_bookmark_list(bookmark_list),
            bookmark_area,
            &mut bookmark_list.table_state,
        );
    } else {
        frame.render_stateful_widget(
            build_filer(&state.filer),
            content_area,
            &mut state.filer.file_table_state,
        );
    }

    frame.render_widget(build_input_area(&state.input), input_area);
}
