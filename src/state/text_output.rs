use crossterm::event::KeyCode;
use ratatui::text::Line;

#[derive(Debug)]
pub struct TextOutputState {
    pub lines: Vec<Line<'static>>,
    pub scroll_offset: u16,
    pub visible_height: u16,
    pub visible_width: u16,
    cached_visual_lines: Vec<u32>,
    cached_total_visual_lines: u32,
}

impl TextOutputState {
    /// プレーンテキスト行から構築する。各行はスタイルなしの `Line` に変換される。
    pub fn with_lines(lines: Vec<String>) -> Self {
        Self::with_styled_lines(lines.into_iter().map(Line::from).collect())
    }

    /// スタイル付きの `Line` から構築する（マークダウン等のレンダリング結果用）。
    pub fn with_styled_lines(lines: Vec<Line<'static>>) -> Self {
        Self {
            lines,
            scroll_offset: 0,
            visible_height: 0,
            visible_width: 0,
            cached_visual_lines: Vec::new(),
            cached_total_visual_lines: 0,
        }
    }

    pub fn set_visible_area(&mut self, height: u16, width: u16) {
        self.visible_height = height;
        if self.visible_width != width {
            self.visible_width = width;
            self.recalculate_visual_lines();
        }
    }

    /// スクロール系キーイベントを処理する。処理した場合は true を返す。
    pub fn handle_scroll_key(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Up => self.scroll_up(),
            KeyCode::Down => self.scroll_down(),
            KeyCode::Left => self.scroll_to_top(),
            KeyCode::Right => self.scroll_to_bottom(),
            _ => return false,
        }
        true
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
        self.cached_visual_lines = self.lines.iter().map(|l| visual_lines(width, l)).collect();
        self.cached_total_visual_lines = self.cached_visual_lines.iter().sum();
    }

    /// 表示に必要な行範囲と、その範囲内でのスクロールオフセットを返す
    pub fn visible_range(&self) -> (usize, usize, u16) {
        let offset = self.scroll_offset as u32;
        let height = self.visible_height as u32;

        let mut visual_row = 0u32;
        let mut start_line = 0;
        let mut start_offset = 0u16;

        // scroll_offset に対応する開始行を探す
        for (i, &vl) in self.cached_visual_lines.iter().enumerate() {
            if visual_row + vl > offset {
                start_line = i;
                start_offset = (offset - visual_row) as u16;
                break;
            }
            visual_row += vl;
            start_line = i + 1;
        }

        // visible_height 分の終了行を探す
        let target_end = offset + height;
        let mut end_line = start_line;
        for &vl in self.cached_visual_lines.iter().skip(start_line) {
            if visual_row >= target_end {
                break;
            }
            visual_row += vl;
            end_line += 1;
        }

        (start_line, end_line, start_offset)
    }

    fn clamp_scroll(&mut self) {
        let max = self.max_scroll();
        if self.scroll_offset > max {
            self.scroll_offset = max;
        }
    }
}

fn visual_lines(visible_width: u16, line: &Line) -> u32 {
    if visible_width == 0 {
        return 1;
    }
    let len = line.width() as u32;
    if len == 0 {
        1
    } else {
        len.div_ceil(visible_width as u32)
    }
}
