mod app;
mod app_context;
mod component;
mod event;
mod fs;
mod os;
mod state;
mod store;
mod ui;

use anyhow::Result;
use app::App;
use ratatui_image::picker::{Picker, ProtocolType};

fn main() -> Result<()> {
    // ターミナルの画像プロトコルを検出（alternate screen に入る前に実行する必要がある）
    let mut picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
    // クエリ応答がコンソールに残るのを消去する
    print!("\r\x1b[2K");
    // iTerm2 は Kitty プロトコルのクエリに応答するが完全にはサポートしていないため、
    // ネイティブの iTerm2 プロトコルを使用する。
    // KITTY_WINDOW_ID が設定されている場合は実際の Kitty ターミナルなので上書きしない。
    let is_kitty = std::env::var("KITTY_WINDOW_ID").is_ok();
    let is_iterm = std::env::var("TERM_PROGRAM").is_ok_and(|p| p.contains("iTerm"));
    if is_iterm && !is_kitty {
        picker.set_protocol_type(ProtocolType::Iterm2);
    }
    // FV_IMAGE_PROTOCOL が指定されていれば、自動検出や iTerm2 判定を上書きする。
    // ttyd / xterm.js 系のように対応していないプロトコルを誤検出する端末で、
    // halfblocks 等を明示指定して描画させるための手段。
    // 未設定・未知の値は無視して自動検出のままにする。
    if let Ok(value) = std::env::var("FV_IMAGE_PROTOCOL")
        && let Some(protocol) = parse_image_protocol(&value)
    {
        picker.set_protocol_type(protocol);
    }

    let mut terminal = ratatui::init();
    let mut app = App::new(picker)?;
    app.init()?;
    let result = app.run(&mut terminal);

    ratatui::restore();
    result
}

/// `FV_IMAGE_PROTOCOL` の値を `ProtocolType` に変換する。
/// 大文字小文字は無視し、未知の値・空文字には `None` を返す（自動検出にフォールバック）。
fn parse_image_protocol(value: &str) -> Option<ProtocolType> {
    match value.trim().to_ascii_lowercase().as_str() {
        "halfblocks" => Some(ProtocolType::Halfblocks),
        "sixel" => Some(ProtocolType::Sixel),
        "kitty" => Some(ProtocolType::Kitty),
        "iterm2" => Some(ProtocolType::Iterm2),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_image_protocol_maps_known_values_case_insensitively() {
        assert_eq!(
            parse_image_protocol("halfblocks"),
            Some(ProtocolType::Halfblocks)
        );
        assert_eq!(
            parse_image_protocol("HALFBLOCKS"),
            Some(ProtocolType::Halfblocks)
        );
        assert_eq!(parse_image_protocol("Sixel"), Some(ProtocolType::Sixel));
        assert_eq!(parse_image_protocol("kitty"), Some(ProtocolType::Kitty));
        assert_eq!(
            parse_image_protocol("  iterm2  "),
            Some(ProtocolType::Iterm2)
        );
    }

    #[test]
    fn parse_image_protocol_returns_none_for_unknown_or_empty() {
        assert_eq!(parse_image_protocol(""), None);
        assert_eq!(parse_image_protocol("auto"), None);
        assert_eq!(parse_image_protocol("png"), None);
    }
}
