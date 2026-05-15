mod attribute;
mod file_info;

pub use attribute::AttributeComponent;
pub use file_info::FileInfoComponent;

use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

/// アプリ全体に影響するアクション。
/// コンポーネントの `handle_event` が返し、App のメインループで処理する。
#[derive(Debug)]
#[allow(dead_code)]
pub enum Action {
    /// 何もしない
    None,
    /// アプリケーションを終了する
    Quit,
    /// エラーメッセージを表示する
    Error(String),
    /// 外部シェルを起動する
    LaunchShell,
    /// サイドパネルを閉じる
    CloseSidePanel,
}

/// コンポーネントの共通インターフェース。
/// 各エリア（Filer, Prompt, Bookmark 等）がこの trait を実装する。
pub trait Component {
    /// キーイベントを処理し、アプリ全体に影響するアクションを返す。
    fn handle_event(&mut self, event: KeyEvent) -> Result<Action>;

    /// コンポーネントを描画する。
    fn render(&mut self, frame: &mut Frame, area: Rect);
}
