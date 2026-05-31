use crate::app_context::AppContext;
use crate::os::clock::format_clock;
use crate::os::disk_usage::format_disk_field;
use crate::ui::widgets::{BorderState, Focus, build_bordered_block};
use chrono::Local;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::widgets::Paragraph;
use std::path::Path;

/// Clock フィールドの占有幅。`YYYY-MM-DD HH:MM:SS`（19桁）＋左の status との間隔。
const CLOCK_FIELD_WIDTH: u16 = 21;

/// 時計を表示するために左ゾーン（status/disk）へ最低限残す幅。
/// これを割り込むほど内容行が狭いときは時計を省略し、左の情報を優先する。
const MIN_STATUS_WIDTH: u16 = 40;

/// ヘッダー枠を描画する。
/// タイトル（枠線）に静的情報（アプリ名・バージョン ＋ OS/カーネル/ホスト名）を ` | ` 区切りで、
/// 内容行の左ゾーンに動的情報（CPU/メモリ/アップタイム ＋ カレントディレクトリの Disk Usage）を、
/// 右ゾーンに現在時刻（Clock）を表示する。幅が狭いときは左を優先して時計を省略する。
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
    // 左ゾーン: 動的情報＋カレントディレクトリの Disk Usage。
    // 右ゾーン: 現在時刻（Clock）。ただし左に最低幅を残せないほど狭いときは時計を省略する。
    let show_clock = inner.width >= MIN_STATUS_WIDTH + CLOCK_FIELD_WIDTH;
    let (left, clock) = if show_clock {
        let [left, right] =
            Layout::horizontal([Constraint::Min(0), Constraint::Length(CLOCK_FIELD_WIDTH)])
                .areas(inner);
        (left, Some(right))
    } else {
        (inner, None)
    };
    let current_dir = Path::new(ctx.filer.current_dir_path());
    let disk_field = format_disk_field(ctx.disk_usage.usage_for(current_dir));
    let status_line = format!("{}  {disk_field}", info.status_line());
    frame.render_widget(Paragraph::new(status_line), left);
    if let Some(clock) = clock {
        frame.render_widget(
            Paragraph::new(format_clock(Local::now())).alignment(Alignment::Right),
            clock,
        );
    }
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

    /// `YYYY-MM-DD HH:MM:SS` の形をした 19 文字の並びが含まれるか判定する。
    /// Clock の値は `Local::now()` 依存で非決定論的なので、固定値ではなく「形」で照合する。
    fn contains_clock(text: &str) -> bool {
        let chars: Vec<char> = text.chars().collect();
        chars.windows(19).any(|w| {
            w.iter().enumerate().all(|(i, &c)| match i {
                4 | 7 => c == '-',
                10 => c == ' ',
                13 | 16 => c == ':',
                _ => c.is_ascii_digit(),
            })
        })
    }

    #[test]
    fn header_shows_clock_in_body() {
        let ctx = AppContext::new(Picker::halfblocks());
        let text = render_to_string(&ctx, 100);
        assert!(
            contains_clock(&text),
            "body should show a YYYY-MM-DD HH:MM:SS clock: {text:?}"
        );
    }

    #[test]
    fn header_omits_clock_when_too_narrow() {
        let ctx = AppContext::new(Picker::halfblocks());
        // 左の status/disk を優先し、時計を収められない狭い幅では Clock を省略する。
        let text = render_to_string(&ctx, 40);
        assert!(
            !contains_clock(&text),
            "narrow header should omit the clock: {text:?}"
        );
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
        assert!(
            text.contains("Disk "),
            "body should show Disk usage: {text:?}"
        );
    }
}
