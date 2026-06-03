use ratatui::text::{Line, Span, Text};

/// マークダウンソースをレンダリングし、サイドパネルに保持できる
/// `'static` なスタイル付き行へ変換する。
///
/// `tui_markdown::from_str` は入力を借用した `Text<'_>` を返すため、
/// コンポーネントに保持できるよう各 Span を所有文字列へコピーして 'static 化する。
pub fn to_lines(source: &str) -> Vec<Line<'static>> {
    into_owned_lines(tui_markdown::from_str(source))
}

fn into_owned_lines(text: Text<'_>) -> Vec<Line<'static>> {
    text.lines
        .into_iter()
        .map(|line| {
            let spans: Vec<Span<'static>> = line
                .spans
                .into_iter()
                .map(|span| Span::styled(span.content.into_owned(), span.style))
                .collect();
            let mut owned = Line::from(spans);
            owned.style = line.style;
            owned.alignment = line.alignment;
            owned
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Style;

    #[test]
    fn render_produces_owned_styled_lines_for_heading() {
        let lines = to_lines("# Title\n\nbody text\n");
        // 少なくとも見出し行と本文行が生成される。
        assert!(lines.len() >= 2);
        let heading = &lines[0];
        // 見出し行には装飾が付く＝行またはいずれかの Span に非デフォルトのスタイルがある。
        let has_style = heading.style != Style::default()
            || heading.spans.iter().any(|s| s.style != Style::default());
        assert!(has_style, "heading line should carry markdown styling");
        // テキスト内容が保持されている。
        let rendered: String = heading
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<String>();
        assert!(rendered.contains("Title"));
    }

    #[test]
    fn render_handles_lists_and_code_without_panicking() {
        let lines = to_lines("- item one\n- item two\n\n```\ncode\n```\n");
        assert!(!lines.is_empty());
    }
}
