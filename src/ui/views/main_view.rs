use crate::app_context::AppContext;
use crate::component::Component;
use crate::store::RootStore;
use crate::ui::features::build_header;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

/// fv の表示に必要な最小ウィンドウサイズ。これを下回ると通常 UI の代わりに
/// 「ウィンドウを広げて」警告を表示する。
const MIN_WIDTH: u16 = 60;
const MIN_HEIGHT: u16 = 15;

/// 端末サイズが fv の表示に十分かを判定する。
fn meets_minimum_size(area: Rect) -> bool {
    area.width >= MIN_WIDTH && area.height >= MIN_HEIGHT
}

/// ウィンドウが小さすぎるときに通常 UI の代わりに表示する案内。
/// 必要サイズと現在サイズを中央に表示する。
fn render_too_small_warning(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from("Window too small"),
        Line::from(format!("Resize to at least {MIN_WIDTH}x{MIN_HEIGHT}")),
        Line::from(format!("(current: {}x{})", area.width, area.height)),
    ];
    // 行数を lines から導出して二重管理を避ける。
    let text_height = lines.len() as u16;
    let message = Paragraph::new(lines).alignment(Alignment::Center);

    // メッセージを縦方向にも中央へ寄せる。
    let offset = area.height.saturating_sub(text_height) / 2;
    let y = area.y.saturating_add(offset);
    let centered = Rect::new(area.x, y, area.width, text_height.min(area.height));
    frame.render_widget(message, centered);
}

pub fn render_main_view(frame: &mut Frame, ctx: &mut AppContext, store: &RootStore) {
    let area = frame.area();

    // ウィンドウが小さすぎるときは通常 UI を描かず案内のみ表示する。
    // メインループが毎フレーム再描画するため、広げれば自動的に通常 UI へ復帰する。
    if !meets_minimum_size(area) {
        render_too_small_warning(frame, area);
        return;
    }

    let [header_area, content_area, prompt_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(3),
    ])
    .areas(area);

    frame.render_widget(build_header(ctx), header_area);
    match &mut ctx.side_panel {
        Some(panel) => {
            let [filer_area, panel_area] =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(content_area);
            ctx.filer.render_with_store(frame, filer_area, store);
            panel.render(frame, panel_area);
        }
        None => {
            ctx.filer.render_with_store(frame, content_area, store);
        }
    }
    // アイドル時に表示するキーマップは、アクティブなコンポーネント自身が提供する。
    // サイドパネル表示中はそのパネル、さもなくば Filer のキーマップ。
    let keymap = match &ctx.side_panel {
        Some(panel) => panel.keymap(),
        None => ctx.filer.keymap(),
    };
    ctx.prompt.render_with_keymap(frame, prompt_area, keymap);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(width: u16, height: u16) -> Rect {
        Rect::new(0, 0, width, height)
    }

    #[test]
    fn area_below_minimum_is_rejected() {
        assert!(!meets_minimum_size(area(40, 10)));
    }

    #[test]
    fn area_exactly_at_minimum_is_accepted() {
        assert!(meets_minimum_size(area(60, 15)));
    }

    #[test]
    fn area_short_in_only_one_dimension_is_rejected() {
        assert!(!meets_minimum_size(area(60, 10)), "height below minimum");
        assert!(!meets_minimum_size(area(40, 15)), "width below minimum");
    }

    fn render_warning_to_string(width: u16, height: u16) -> String {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("build terminal");
        terminal
            .draw(|frame| {
                let a = frame.area();
                render_too_small_warning(frame, a);
            })
            .expect("draw warning");
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn warning_shows_required_and_current_size() {
        let text = render_warning_to_string(40, 10);

        assert!(
            text.contains("too small"),
            "message expected, got: {text:?}"
        );
        assert!(
            text.contains("60x15"),
            "required size expected, got: {text:?}"
        );
        assert!(
            text.contains("40x10"),
            "current size expected, got: {text:?}"
        );
    }
}
