use crate::component::Component;
use crate::state::{AppState, PromptMode, SidePanel};
use crate::store::RootStore;
use crate::ui::features::{build_filer, build_header, build_path_table, build_prompt_view};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use unicode_width::UnicodeWidthChar;

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
            match panel {
                SidePanel::Attribute(component) => component.render(frame, panel_area),
                SidePanel::FileInfo(component) => component.render(frame, panel_area),
                SidePanel::Bookmark(bookmark) => {
                    frame.render_stateful_widget(
                        build_path_table(bookmark, "Bookmark"),
                        panel_area,
                        &mut bookmark.table_state,
                    );
                }
                SidePanel::Grep(grep) => {
                    frame.render_stateful_widget(
                        build_path_table(grep, "Grep"),
                        panel_area,
                        &mut grep.table_state,
                    );
                }
            }
        }
        None => {
            frame.render_stateful_widget(filer, content_area, &mut state.filer.file_table_state);
        }
    }
    frame.render_widget(build_prompt_view(&state.prompt), prompt_area);

    // プロンプトのテキスト入力時にカーソルを表示
    if let Some(cursor_char_pos) = state.prompt.cursor_position() {
        if let Some(value) = match &state.prompt {
            PromptMode::Text { value, .. }
            | PromptMode::File { value, .. }
            | PromptMode::Search { value, .. } => Some(value.as_str()),
            _ => None,
        } {
            let display_width: usize = value
                .chars()
                .take(cursor_char_pos)
                .map(|c| UnicodeWidthChar::width(c).unwrap_or(0))
                .sum();
            // ボーダー(1) + パディング(1) + 表示幅
            let cursor_x = prompt_area.x + 2 + display_width as u16;
            let cursor_y = prompt_area.y + 1;
            frame.set_cursor_position(ratatui::layout::Position::new(cursor_x, cursor_y));
        }
    }
}
