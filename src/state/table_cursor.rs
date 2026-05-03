use ratatui::widgets::TableState;

pub struct TableCursor<'a> {
    state: &'a mut TableState,
    len: usize,
}

impl<'a> TableCursor<'a> {
    pub fn new(state: &'a mut TableState, len: usize) -> Self {
        Self { state, len }
    }

    pub fn next(&mut self) {
        if self.len == 0 {
            return;
        }
        if let Some(selected) = self.state.selected() {
            if selected < self.len - 1 {
                self.state.select(Some(selected + 1));
            }
        }
    }

    pub fn prev(&mut self) {
        if self.len == 0 {
            return;
        }
        if let Some(selected) = self.state.selected() {
            if selected > 0 {
                self.state.select(Some(selected - 1));
            }
        }
    }

    pub fn first(&mut self) {
        if self.len == 0 {
            return;
        }
        self.state.select(Some(0));
    }

    pub fn last(&mut self) {
        if self.len == 0 {
            return;
        }
        self.state.select(Some(self.len - 1));
    }
}
