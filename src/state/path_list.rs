use crate::state::table_cursor::TableCursor;
use ratatui::widgets::TableState;
use std::sync::mpsc::{Receiver, TryRecvError};

#[derive(Debug)]
pub struct PathListState {
    pub table_state: TableState,
    pub paths: Vec<String>,
    pub rx: Option<Receiver<String>>,
}

impl PathListState {
    pub fn new(paths: Vec<String>, rx: Option<Receiver<String>>) -> Self {
        let mut table_state = TableState::default();
        if !paths.is_empty() {
            table_state.select(Some(0));
        }
        Self {
            table_state,
            paths,
            rx,
        }
    }

    fn cursor(&mut self) -> TableCursor {
        TableCursor::new(&mut self.table_state, self.paths.len())
    }

    pub fn next(&mut self) {
        self.cursor().next();
    }

    pub fn prev(&mut self) {
        self.cursor().prev();
    }

    pub fn first(&mut self) {
        self.cursor().first();
    }

    pub fn last(&mut self) {
        self.cursor().last();
    }

    pub fn selected_path(&self) -> Option<&str> {
        self.table_state
            .selected()
            .and_then(|i| self.paths.get(i).map(String::as_str))
    }

    pub fn remove(&mut self, path: &str) {
        self.paths.retain(|p| p != path);
        if let Some(selected) = self.table_state.selected() {
            if self.paths.is_empty() {
                self.table_state.select(None);
            } else if selected >= self.paths.len() {
                self.table_state.select(Some(self.paths.len() - 1));
            }
        }
    }

    pub fn is_running(&self) -> bool {
        self.rx.is_some()
    }

    pub fn receive_results(&mut self) {
        let Some(rx) = &mut self.rx else {
            return;
        };

        const MAX_RECV_PER_FRAME: usize = 100;

        let mut count = 0;
        loop {
            if count >= MAX_RECV_PER_FRAME {
                break;
            }
            match rx.try_recv() {
                Ok(path) => {
                    self.paths.push(path);
                    count += 1;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.rx = None;
                    break;
                }
            }
        }

        if count > 0 && self.table_state.selected().is_none() {
            self.table_state.select(Some(0));
        }
    }
}
