//! テキストプレビューのシンタックスハイライト。
//! `syntect` をこのモジュールに閉じ込め、ファイル種別に応じて色付けした
//! `'static` なスタイル付き行（`Vec<Line<'static>>`）を返す。
//!
//! 配色は tui-markdown のコードブロックと揃え、テーマ `base16-ocean.dark` の
//! 前景色のみを使う（背景は端末に委ねる）。マークダウンの「レンダリング」とは異なり、
//! ここでは生テキストをそのまま色付けする。

use std::path::Path;
use std::sync::LazyLock;

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style as SyntectStyle, Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};

/// 構文定義一式。改行付き行を前提とする `newlines` 版を使う（`highlight_line` に渡す
/// 各行へ末尾改行を付けるため）。初回アクセス時に一度だけ読み込む。
static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);

/// 配色テーマ。tui-markdown のコードブロックと同じ `base16-ocean.dark` を使い、
/// md 内コードとソースファイル単体で配色を一貫させる。
static THEME: LazyLock<Theme> =
    LazyLock::new(|| ThemeSet::load_defaults().themes["base16-ocean.dark"].clone());

/// 行群を構文ハイライトして `'static` なスタイル付き行へ変換する。
/// ファイル種別を特定できなければ `None` を返す（呼び出し側はプレーン表示にフォールバックする）。
///
/// 種別判定は拡張子（無ければファイル名トークン）→ 先頭行（shebang 等）の順。
pub fn to_lines(lines: &[String], file_name: &str) -> Option<Vec<Line<'static>>> {
    let first_line = lines.first().map(String::as_str);
    let syntax = find_syntax(file_name, first_line)?;

    let syntax_set = &SYNTAX_SET;
    let mut highlighter = HighlightLines::new(syntax, &THEME);
    let out = lines
        .iter()
        .map(|line| highlight_line(&mut highlighter, syntax_set, line))
        .collect();
    Some(out)
}

/// 1 行をハイライトして所有スタイル付き行へ変換する。失敗時はプレーン行にフォールバックする。
fn highlight_line(
    highlighter: &mut HighlightLines<'_>,
    syntax_set: &SyntaxSet,
    line: &str,
) -> Line<'static> {
    // `newlines` 構文セットは行末の改行を前提とするため付与し、Span へは含めない。
    let with_newline = format!("{line}\n");
    match highlighter.highlight_line(&with_newline, syntax_set) {
        Ok(ranges) => {
            let spans: Vec<Span<'static>> = ranges
                .into_iter()
                .filter_map(|(style, text)| {
                    let content = text.trim_end_matches('\n');
                    (!content.is_empty())
                        .then(|| Span::styled(content.to_string(), to_ratatui_style(style)))
                })
                .collect();
            Line::from(spans)
        }
        Err(_) => Line::from(line.to_string()),
    }
}

/// ファイル名（拡張子・ファイル名トークン）と先頭行から構文定義を解決する。
fn find_syntax(file_name: &str, first_line: Option<&str>) -> Option<&'static SyntaxReference> {
    let syntax_set = &SYNTAX_SET;
    let path = Path::new(file_name);
    // 拡張子で照合。拡張子が無いファイル（Makefile / Dockerfile 等）はファイル名トークンで照合する。
    let token = path
        .extension()
        .and_then(|e| e.to_str())
        .or_else(|| path.file_name().and_then(|n| n.to_str()));
    if let Some(token) = token
        && let Some(syntax) = syntax_set.find_syntax_by_extension(token)
    {
        return Some(syntax);
    }
    // 拡張子で特定できなければ先頭行（shebang 等）で照合する。
    first_line.and_then(|first| syntax_set.find_syntax_by_first_line(first))
}

/// syntect のスタイルを ratatui のスタイルへ変換する。配色一貫性のため前景色のみを使う。
fn to_ratatui_style(style: SyntectStyle) -> Style {
    let fg = style.foreground;
    Style::default().fg(Color::Rgb(fg.r, fg.g, fg.b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlights_rust_source_by_extension() {
        let lines = vec!["fn main() {}".to_string()];
        let result = to_lines(&lines, "main.rs").expect("rust should be detected");
        // 少なくとも 1 つの Span に前景色が付く＝ハイライトされている。
        let has_color = result
            .iter()
            .flat_map(|l| l.spans.iter())
            .any(|s| s.style.fg.is_some());
        assert!(has_color, "rust source should be colored");
    }

    #[test]
    fn detects_language_from_shebang_without_extension() {
        let lines = vec!["#!/bin/bash".to_string(), "echo hi".to_string()];
        assert!(
            to_lines(&lines, "myscript").is_some(),
            "shebang should be detected via first line"
        );
    }

    #[test]
    fn returns_none_for_unknown_type() {
        let lines = vec!["just some plain prose".to_string()];
        assert!(
            to_lines(&lines, "notes.unknownext").is_none(),
            "unknown extension with no shebang should fall back to plain"
        );
    }

    #[test]
    fn line_count_is_preserved() {
        let lines = vec![
            "fn main() {".to_string(),
            "".to_string(),
            "    let x = 1;".to_string(),
            "}".to_string(),
        ];
        let result = to_lines(&lines, "main.rs").expect("rust should be detected");
        assert_eq!(result.len(), lines.len());
    }
}
