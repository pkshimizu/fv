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

pub fn move_first(table_state: &mut TableState, len: usize) {
    if len == 0 {
        return;
    }
    table_state.select(Some(0));
}

pub fn move_last(table_state: &mut TableState, len: usize) {
    if len == 0 {
        return;
    }
    table_state.select(Some(len - 1));
}
