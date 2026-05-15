mod attribute;
mod bookmark;
mod file_info;
mod grep;
mod prompt;

pub use attribute::AttributeComponent;
pub use bookmark::BookmarkComponent;
pub use file_info::FileInfoComponent;
pub use grep::GrepComponent;
pub use prompt::PromptComponent;

use crate::state::PromptMode;
use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

/// アプリ全体に影響するアクション。
/// コンポーネントの `handle_event` が返し、App のメインループで処理する。
#[derive(Debug)]
pub enum Action {
    /// 何もしない
    None,
    /// アプリケーションを終了する（Filer コンポーネント化時に使用予定）
    #[allow(dead_code)]
    Quit,
    /// エラーメッセージを表示する（Filer コンポーネント化時に使用予定）
    #[allow(dead_code)]
    Error(String),
    /// 外部シェルを起動する
    LaunchShell,
    /// サイドパネルを閉じる
    CloseSidePanel,
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
    fn render(&mut self, frame: &mut Frame, area: Rect);

    /// 毎フレーム呼ばれるライフサイクルメソッド。非同期結果の受信等に使用する。
    fn tick(&mut self) {}
}
