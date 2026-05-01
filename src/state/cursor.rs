use ratatui::widgets::TableState;

pub fn move_next(table_state: &mut TableState, len: usize) {
    if len == 0 {
        return;
    }
    if let Some(selected) = table_state.selected() {
        if selected < len - 1 {
            table_state.select(Some(selected + 1));
        }
    }
}

pub fn move_prev(table_state: &mut TableState, len: usize) {
    if len == 0 {
        return;
    }
    if let Some(selected) = table_state.selected() {
        if selected > 0 {
            table_state.select(Some(selected - 1));
        }
    }
}
