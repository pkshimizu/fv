use crate::state::{AppState, SidePanel};
use crate::store::RootStore;
use crate::ui::features::{
    build_attribute_table, build_filer, build_header, build_path_table, build_prompt_view,
};
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
        Some(SidePanel::Attribute(attribute)) => {
            let [filer_area, attribute_area] =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(content_area);
            frame.render_stateful_widget(filer, filer_area, &mut state.filer.file_table_state);
            frame.render_stateful_widget(
                build_attribute_table(attribute),
                attribute_area,
                &mut attribute.table_state,
            )
        }
        Some(SidePanel::Bookmark(bookmark)) => {
            let [filer_area, bookmark_area] =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(content_area);
            frame.render_stateful_widget(filer, filer_area, &mut state.filer.file_table_state);
            frame.render_stateful_widget(
                build_path_table(bookmark, "Bookmark"),
                bookmark_area,
                &mut bookmark.table_state,
            )
        }
        Some(SidePanel::Grep(grep)) => {
            let [filer_area, grep_area] =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(content_area);
            frame.render_stateful_widget(filer, filer_area, &mut state.filer.file_table_state);
            frame.render_stateful_widget(
                build_path_table(grep, "Grep"),
                grep_area,
                &mut grep.table_state,
            )
        }
        None => {
            frame.render_stateful_widget(filer, content_area, &mut state.filer.file_table_state);
        }
    }
    frame.render_widget(build_prompt_view(&state.prompt), prompt_area);
}
