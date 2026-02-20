use crate::state::AppState;
use crate::ui::features::{build_filer, build_header};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};

pub fn render_main_view(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    let [header_area, filter_area] =
        Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).areas(area);

    frame.render_widget(build_header(state), header_area);
    frame.render_widget(build_filer(state), filter_area);
}
