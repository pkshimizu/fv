use crate::app_context::AppContext;
use crate::os::disk_usage::format_disk_field;
use crate::ui::widgets::{BorderState, Focus, build_bordered_block};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;
use std::path::Path;

/// ヘッダー枠を描画する。
/// タイトル（枠線）に静的情報（アプリ名・バージョン ＋ OS/カーネル/ホスト名）を ` | ` 区切りで、
/// 内容行の左ゾーンに動的情報（CPU/メモリ/アップタイム ＋ カレントディレクトリの Disk Usage）を表示する。
/// 右ゾーンは将来の時刻表示用に空けておく。
pub fn render_header(frame: &mut Frame, ctx: &AppContext, area: Rect) {
    let info = ctx.system_info.current();
    // アプリ名・バージョンは Cargo.toml から取得する。
    let title = format!(
        "{}<{}> | {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        info.title_fields()
    );
    let block = build_bordered_block(&title, Focus::Unfocused, BorderState::Normal);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    // 左ゾーン: 動的情報＋カレントディレクトリの Disk Usage。右ゾーン（時刻）は将来追加する。
    let current_dir = Path::new(ctx.filer.current_dir_path());
    let disk_field = format_disk_field(ctx.disk_usage.usage_for(current_dir));
    let status_line = format!("{}  {disk_field}", info.status_line());
    frame.render_widget(Paragraph::new(status_line), inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_context::AppContext;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui_image::picker::Picker;

    fn render_to_string(ctx: &AppContext, width: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, 3)).expect("terminal");
        terminal
            .draw(|frame| render_header(frame, ctx, frame.area()))
            .expect("draw");
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn header_shows_app_name_in_title_and_dynamic_labels_in_body() {
        let ctx = AppContext::new(Picker::halfblocks());
        let text = render_to_string(&ctx, 100);

        // タイトルにアプリ名（Cargo メタ）。値は環境依存だが `fv<` の形は安定。
        assert!(text.contains("fv<"), "title should show app name: {text:?}");
        // 内容行に動的情報のラベル。
        assert!(text.contains("CPU "), "body should show CPU: {text:?}");
        assert!(text.contains("Mem "), "body should show Mem: {text:?}");
        assert!(text.contains("up "), "body should show uptime: {text:?}");
        // カレントディレクトリの Disk Usage も内容行に表示される（値・単位は環境依存だが
        // `Disk ` ラベルは常に出る。未特定時は `Disk n/a`）。
        assert!(text.contains("Disk "), "body should show Disk usage: {text:?}");
    }
}
