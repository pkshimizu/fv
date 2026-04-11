use crate::state::{AppState, ModalState};
use crate::ui::features::{build_filer, build_header, render_delete_confirm_modal};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};

pub fn render_main_view(frame: &mut Frame, state: &mut AppState) {
    let area = frame.area();

    let [header_area, filer_area] =
        Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).areas(area);

    frame.render_widget(build_header(state), header_area);
    frame.render_stateful_widget(
        build_filer(&state.filer),
        filer_area,
        &mut state.filer.file_table_state,
    );

    match &state.modal {
        ModalState::None => {}
        ModalState::DeleteConfirm => {
            let file = state.filer.selected_file();
            if let Some(file) = file {
                render_delete_confirm_modal(frame, area, &file);
            }
        }
    }
}
