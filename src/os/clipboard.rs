use anyhow::{Context, Result};

/// Operation Targets の絶対パス列をシステムクリップボードへ書き出す（Yank）。
/// パスは `\n` 区切り（末尾改行なし）。空のときは何もしない
/// （空文字でクリップボードを上書きしない）。
pub fn write_paths(paths: &[String]) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    let text = format_paths(paths);
    arboard::Clipboard::new()
        .context("Failed to access clipboard")?
        .set_text(text)
        .context("Failed to write to clipboard")
}

/// Operation Targets の絶対パス列を、クリップボードへ書き込む 1 つの文字列に整形する。
/// パスは `\n` で join し、末尾には改行を付けない（端末プロンプトへの貼り付けで
/// 最後のパスが誤ってコマンド実行されるのを防ぐ）。
fn format_paths(paths: &[String]) -> String {
    paths.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_paths_joins_with_newline_and_no_trailing_newline() {
        let paths = vec![
            "/a/one.txt".to_string(),
            "/a/two.txt".to_string(),
            "/a/three.txt".to_string(),
        ];

        assert_eq!(format_paths(&paths), "/a/one.txt\n/a/two.txt\n/a/three.txt");
    }

    #[test]
    fn format_paths_of_a_single_path_has_no_newline() {
        let paths = vec!["/a/only.txt".to_string()];

        assert_eq!(format_paths(&paths), "/a/only.txt");
    }

    #[test]
    fn format_paths_of_empty_is_empty_string() {
        let paths: Vec<String> = Vec::new();

        assert_eq!(format_paths(&paths), "");
    }
}
