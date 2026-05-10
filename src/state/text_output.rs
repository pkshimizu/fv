use std::sync::mpsc::{Receiver, TryRecvError};
use unicode_width::UnicodeWidthStr;

#[derive(Debug)]
pub struct TextOutputState {
    pub lines: Vec<String>,
    pub scroll_offset: u16,
    pub visible_height: u16,
    pub visible_width: u16,
    pub rx: Option<Receiver<String>>,
    cached_total_visual_lines: u32,
}

impl TextOutputState {
    pub fn new(rx: Option<Receiver<String>>) -> Self {
        Self {
            lines: Vec::new(),
            scroll_offset: 0,
            visible_height: 0,
            visible_width: 0,
            rx,
            cached_total_visual_lines: 0,
        }
    }

    pub fn is_running(&self) -> bool {
        self.rx.is_some()
    }

    pub fn set_visible_area(&mut self, height: u16, width: u16) {
        self.visible_height = height;
        if self.visible_width != width {
            self.visible_width = width;
            self.recalculate_visual_lines();
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
        self.clamp_scroll();
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.max_scroll();
    }

    fn max_scroll(&self) -> u16 {
        let total = self.total_visual_lines();
        total.saturating_sub(self.visible_height)
    }

    fn total_visual_lines(&self) -> u16 {
        self.cached_total_visual_lines.min(u16::MAX as u32) as u16
    }

    fn recalculate_visual_lines(&mut self) {
        let width = self.visible_width;
        self.cached_total_visual_lines =
            self.lines.iter().map(|l| visual_lines(width, l)).sum();
    }

    fn clamp_scroll(&mut self) {
        let max = self.max_scroll();
        if self.scroll_offset > max {
            self.scroll_offset = max;
        }
    }

    pub fn receive_results(&mut self) {
        let Some(rx) = &mut self.rx else {
            return;
        };

        const MAX_RECV_PER_FRAME: usize = 100;
        const MAX_LINES: usize = 10000;
        let width = self.visible_width;

        let mut count = 0;
        loop {
            if count >= MAX_RECV_PER_FRAME {
                break;
            }
            if self.lines.len() >= MAX_LINES {
                self.rx = None;
                break;
            }
            match rx.try_recv() {
                Ok(line) => {
                    self.cached_total_visual_lines += visual_lines(width, &line);
                    self.lines.push(line);
                    count += 1;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.rx = None;
                    break;
                }
            }
        }
    }
}

fn visual_lines(visible_width: u16, line: &str) -> u32 {
    if visible_width == 0 {
        return 1;
    }
    let len = line.width() as u32;
    if len == 0 { 1 } else { len.div_ceil(visible_width as u32) }
}
