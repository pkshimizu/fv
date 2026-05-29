/// 非同期処理中に「アプリが生きている」ことを示す Activity Indicator の
/// アニメーション状態。点字10フレームを巡回する。`tick()` 相当のタイミングで
/// `advance()` し、描画時に `frame()` を読む。
pub struct Spinner {
    index: usize,
}

const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

impl Spinner {
    pub fn new() -> Self {
        Self { index: 0 }
    }

    pub fn frame(&self) -> &'static str {
        FRAMES[self.index]
    }

    pub fn advance(&mut self) {
        self.index = (self.index + 1) % FRAMES.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_spinner_shows_the_first_frame() {
        let spinner = Spinner::new();

        assert_eq!(spinner.frame(), "⠋");
    }

    #[test]
    fn advance_moves_to_the_next_frame() {
        let mut spinner = Spinner::new();

        spinner.advance();

        assert_eq!(spinner.frame(), "⠙");
    }

    #[test]
    fn advance_wraps_around_after_the_last_frame() {
        let mut spinner = Spinner::new();

        // 10 フレームをちょうど一周させると先頭に戻る。
        for _ in 0..10 {
            spinner.advance();
        }

        assert_eq!(spinner.frame(), "⠋");
    }
}
