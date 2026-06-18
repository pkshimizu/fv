use crate::app_context::AppContext;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::path::Path;

/// Context（タブ）一覧を 1 行で描画する。各タブは「番号:ディレクトリ名」で表し、
/// アクティブ Context を反転表示する。Context が複数あるときだけ呼ばれる想定。
/// 幅に収まらないぶんは描画時に右端でクリップされる（崩れない）。
pub fn render_tab_bar(frame: &mut Frame, ctx: &AppContext, area: Rect) {
    let active = ctx.active_index();
    let dirs = ctx.context_dirs();
    let mut spans: Vec<Span> = Vec::with_capacity(dirs.len());
    for (i, dir) in dirs.iter().enumerate() {
        let label = format!(" {}:{} ", i + 1, tab_name(dir));
        let style = if i == active {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        spans.push(Span::styled(label, style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// タブに表示するディレクトリの短縮名（パスの末尾要素）。ルートなど末尾要素を
/// 取れない場合はパスそのものを返す。
fn tab_name(dir: &str) -> &str {
    Path::new(dir)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_name_uses_last_path_component() {
        assert_eq!(tab_name("/home/user/projects"), "projects");
        assert_eq!(tab_name("/home/user"), "user");
    }

    #[test]
    fn tab_name_falls_back_to_path_for_root() {
        // ルートは末尾要素が取れないのでパスそのもの。
        assert_eq!(tab_name("/"), "/");
    }
}
