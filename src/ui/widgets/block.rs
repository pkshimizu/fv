use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, BorderType, Padding};

/// ビューがキー入力のフォーカスを持つか（Focused View か否か）。border の線種で表現する。
#[derive(Debug, Clone, Copy)]
pub enum Focus {
    Focused,
    Unfocused,
}

/// ビューの意味状態。border の色で表現する。フォーカス軸とは直交する。
#[derive(Debug, Clone, Copy)]
pub enum BorderState {
    Normal,
    Error,
}

/// 枠付きブロックを構築する。フォーカス（線種）と状態（色）の2軸を独立に受け取る。
/// Focused View は太線（Thick）、unfocused は細線（Plain）。色は通常 Reset、Error のみ Red。
pub fn build_bordered_block(title: &str, focus: Focus, state: BorderState) -> Block<'static> {
    let border_type = match focus {
        Focus::Focused => BorderType::Thick,
        Focus::Unfocused => BorderType::Plain,
    };
    let fg_color = match state {
        BorderState::Normal => Color::Reset,
        BorderState::Error => Color::Red,
    };
    Block::bordered()
        .title(title.to_string())
        .border_type(border_type)
        .border_style(Style::default().fg(fg_color))
        .padding(Padding::horizontal(1))
}

/// Focused View 用の通常状態ブロック。最頻ケースのショートハンド。
/// 開いている間は常に Focused View であるビュー（サイドパネル、アクティブな Prompt 入力など）が使う。
/// `build_bordered_block(title, Focus::Focused, BorderState::Normal)` と同義。
pub fn build_focused_block(title: &str) -> Block<'static> {
    build_bordered_block(title, Focus::Focused, BorderState::Normal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    /// Block を描画し、左上角セルの字形を返す（線種の検証用）。
    fn top_left_symbol(block: Block<'static>) -> String {
        let mut terminal =
            Terminal::new(TestBackend::new(10, 3)).expect("TestBackend terminal の生成に失敗");
        terminal
            .draw(|frame| frame.render_widget(block, frame.area()))
            .expect("block の描画に失敗");
        terminal.backend().buffer()[(0, 0)].symbol().to_string()
    }

    /// Block を描画し、左上角セルの前景色を返す（色の検証用）。
    fn top_left_fg(block: Block<'static>) -> Option<Color> {
        let mut terminal =
            Terminal::new(TestBackend::new(10, 3)).expect("TestBackend terminal の生成に失敗");
        terminal
            .draw(|frame| frame.render_widget(block, frame.area()))
            .expect("block の描画に失敗");
        terminal.backend().buffer()[(0, 0)].style().fg
    }

    #[test]
    fn focused_view_has_thick_border() {
        let block = build_bordered_block("title", Focus::Focused, BorderState::Normal);
        assert_eq!(top_left_symbol(block), "┏");
    }

    #[test]
    fn unfocused_view_has_plain_border() {
        let block = build_bordered_block("title", Focus::Unfocused, BorderState::Normal);
        assert_eq!(top_left_symbol(block), "┌");
    }

    #[test]
    fn error_state_has_red_border() {
        let block = build_bordered_block("title", Focus::Focused, BorderState::Error);
        assert_eq!(top_left_fg(block), Some(Color::Red));
    }

    #[test]
    fn normal_state_keeps_default_color() {
        let block = build_bordered_block("title", Focus::Focused, BorderState::Normal);
        assert_ne!(top_left_fg(block), Some(Color::Red));
    }

    #[test]
    fn focus_and_state_are_orthogonal() {
        // フォーカス（線種）と状態（色）は独立に効く: Focused + Error は太線かつ赤。
        let mut terminal =
            Terminal::new(TestBackend::new(10, 3)).expect("TestBackend terminal の生成に失敗");
        terminal
            .draw(|frame| {
                let block = build_bordered_block("title", Focus::Focused, BorderState::Error);
                frame.render_widget(block, frame.area());
            })
            .expect("block の描画に失敗");
        let cell = terminal.backend().buffer()[(0, 0)].clone();
        assert_eq!(cell.symbol(), "┏", "Focused は太線");
        assert_eq!(cell.style().fg, Some(Color::Red), "Error は赤");
    }

    #[test]
    fn focused_block_helper_is_thick_and_normal() {
        // build_focused_block は Focus::Focused + BorderState::Normal のショートハンド。
        let mut terminal =
            Terminal::new(TestBackend::new(10, 3)).expect("TestBackend terminal の生成に失敗");
        terminal
            .draw(|frame| frame.render_widget(build_focused_block("title"), frame.area()))
            .expect("block の描画に失敗");
        let cell = terminal.backend().buffer()[(0, 0)].clone();
        assert_eq!(cell.symbol(), "┏", "Focused は太線");
        assert_ne!(cell.style().fg, Some(Color::Red), "Normal は赤でない");
    }
}
