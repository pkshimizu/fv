mod attribute;
mod audio_player;
mod bookmark;
mod file_info;
mod filer;
mod grep;
mod help;
mod image_preview;
mod preview;
pub mod prompt;
mod settings;
mod tree;

pub use attribute::AttributeComponent;
pub use audio_player::AudioPlayerComponent;
pub use bookmark::BookmarkComponent;
pub use file_info::FileInfoComponent;
pub use filer::FilerComponent;
pub use grep::GrepComponent;
pub use help::HelpComponent;
pub use image_preview::ImagePreviewComponent;
pub use preview::PreviewComponent;
pub use prompt::PromptComponent;
pub use settings::SettingsComponent;
pub use tree::TreeComponent;

use crate::state::{PromptMode, SidePanel};
use crate::store::StartupDirectory;
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
    /// 外部シェルを起動する
    LaunchShell,
    /// サイドパネルを閉じる
    CloseSidePanel,
    /// プロンプトモードを設定する
    SetPromptMode(Box<PromptMode>),
    /// サイドパネルを表示する
    ShowSidePanel(Box<SidePanel>),
    /// ブックマークを追加する
    AddBookmark(String),
    /// ファイルを外部アプリケーションで開く
    OpenFile(String),
    /// Operation Targets の絶対パスをクリップボードへ書き出す（Yank）
    Yank(Vec<String>),
    /// ブックマーク一覧を表示する
    ShowBookmark,
    /// 設定画面を表示する
    ShowSettings,
    /// 設定を保存する
    SaveSettings(Box<StartupDirectory>),
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
    /// シェルでコマンドを実行する（コマンド文字列, 作業ディレクトリ）
    ExecuteCommand(String, String),
    /// ディレクトリ履歴を一つ戻る
    NavigateBack,
    /// ディレクトリ履歴を一つ進む
    NavigateForward,
    /// プレビュー表示中に次のエントリへ移動する
    PreviewNext,
    /// プレビュー表示中に前のエントリへ移動する
    PreviewPrev,
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

    /// アイドル時（`PromptMode::None`）に Prompt 領域へ表示する操作キーのヒント。
    /// デフォルトは空。表示対象となる Filer と各サイドパネルが override する。
    ///
    /// 体裁: 各項目は `キー: 説明`、項目間は半角スペース2個区切り。末尾は閉じ操作
    /// （`<key>/Esc: Close`。保存等を伴う場合は `Save & Close` のように明記してよい）。
    /// `↑↓` はカーソル移動を `Move`、テキストスクロールを `Scroll` と表記し分ける。
    fn keymap(&self) -> &'static str {
        ""
    }
}
