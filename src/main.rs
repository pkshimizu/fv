mod app;
mod app_context;
mod component;
mod config;
mod event;
mod fs;
mod state;
mod store;
mod ui;

use anyhow::Result;
use app::App;
use config::Config;
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

    let mut terminal = ratatui::init();
    let mut app = App::new(Config::default(), picker)?;
    app.init()?;
    let result = app.run(&mut terminal);

    ratatui::restore();
    result
}
