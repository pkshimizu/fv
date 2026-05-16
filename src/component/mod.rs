mod attribute;
mod bookmark;
mod file_info;
mod filer;
mod grep;
pub mod prompt;

pub use attribute::AttributeComponent;
pub use bookmark::BookmarkComponent;
pub use file_info::FileInfoComponent;
pub use filer::FilerComponent;
pub use grep::GrepComponent;
pub use prompt::PromptComponent;

use crate::state::{PromptMode, SidePanel};
use anyhow::Result;
pub use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

/// アプリ全体に影響するアクション。
/// コンポーネントの `handle_event` が返し、App のメインループで処理する。
pub enum Action {
    /// 何もしない
    None,
    /// アプリケーションを終了する
    Quit,
    /// エラーメッセージを表示する
    #[allow(dead_code)]
    Error(String),
    /// 外部シェルを起動する
    LaunchShell,
    /// サイドパネルを閉じる
    CloseSidePanel,
    /// プロンプトモードを設定する
    SetPromptMode(Box<PromptMode>),
    /// サイドパネルを表示する
    ShowSidePanel(SidePanel),
    /// ブックマークを追加する
    AddBookmark(String),
    /// ファイルを外部アプリケーションで開く
    OpenFile(String),
    /// ブックマーク一覧を表示する
    ShowBookmark,
    /// パスに遷移する（ファイルならディレクトリ移動+選択、ディレクトリなら移動）
    NavigateTo(String),
    /// ブックマークを削除する
    RemoveBookmark(String),
    /// プロンプトの確定アクションを実行する
    ExecutePrompt(Box<PromptMode>),
    /// プロンプトをキャンセルする（Searchモードのカーソル復元含む）
    CancelPrompt,
    /// 検索の次の結果に移動する
    SearchNext(String),
    /// 検索の前の結果に移動する
    SearchPrev(String),
    /// インクリメンタル検索（入力値変更時）
    SearchUpdate(String),
}

/// コンポーネントの共通インターフェース。
/// 各エリア（Filer, Prompt, Bookmark 等）がこの trait を実装する。
pub trait Component {
    /// キーイベントを処理し、アプリ全体に影響するアクションを返す。
    fn handle_event(&mut self, event: KeyEvent) -> Result<Action>;

    /// コンポーネントを描画する。
    /// デフォルトは空実装。描画に追加のコンテキストが必要なコンポーネントは
    /// 独自の描画メソッド（例: render_with_store）を使用する。
    fn render(&mut self, _frame: &mut Frame, _area: Rect) {}

    /// 毎フレーム呼ばれるライフサイクルメソッド。非同期結果の受信等に使用する。
    fn tick(&mut self) {}
}
