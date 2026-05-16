# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`fv` is a Rust binary project using Rust edition 2024.

## Common Commands

```bash
cargo build           # Build
cargo run             # Run
cargo test            # Run all tests
cargo test <name>     # Run a single test by name
cargo clippy          # Lint
cargo fmt             # Format code
cargo check           # Type-check without building
cargo build --release # Release build
```

## Architecture

Single-binary Rust project. Entry point is `src/main.rs`.

### Component Architecture

本プロジェクトは Component Architecture パターンに基づいて設計されています。
各エリア（Filer, Prompt, Bookmark 等）が `Component` trait を実装し、
イベント処理・状態・描画を自己完結的に持ちます。

```
KeyEvent → Component::handle_event → Action → App::handle_action → 状態更新
                                                     ↓
                                        Component::render → UI描画
```

### Directory Structure

```
src/
├── main.rs              # エントリーポイント
├── app.rs               # アプリケーションのメインループ、Action処理
├── app_context.rs       # AppContext（コンポーネントコンテナ）
├── config.rs            # 設定関連
├── event.rs             # イベント取得（EventHandler、InputEvent）
├── component/           # コンポーネント（イベント処理 + 状態 + 描画）
│   ├── mod.rs           # Component trait、Action enum 定義
│   ├── filer.rs         # ファイル一覧（FilerComponent）
│   ├── prompt.rs        # プロンプト入力（PromptComponent）+ 確定アクション実行
│   ├── attribute.rs     # ファイル属性パネル（AttributeComponent）
│   ├── file_info.rs     # ファイル情報パネル（FileInfoComponent）
│   ├── bookmark.rs      # ブックマークパネル（BookmarkComponent）
│   └── grep.rs          # Grepパネル（GrepComponent）
├── state/               # データ型定義
│   ├── mod.rs
│   ├── filer.rs         # FilerState（ファイル一覧データ）
│   ├── prompt.rs        # PromptMode enum、関連型
│   ├── side_panel.rs    # SidePanel enum（Component trait 実装）
│   ├── path_list.rs     # PathListState（リスト系パネル共通）
│   ├── text_output.rs   # TextOutputState（テキスト表示共通）
│   └── table_cursor.rs  # TableCursor（テーブルカーソル共通）
├── ui/                  # UI描画ヘルパー
│   ├── mod.rs
│   ├── views/
│   │   └── main_view.rs # メインビュー（レイアウト構成）
│   ├── features/
│   │   └── header.rs    # ヘッダー描画
│   └── widgets/
│       └── block.rs     # 共通ウィジェット
├── fs/                  # ファイルシステム操作
│   ├── mod.rs
│   ├── file.rs          # VFile（ファイル操作）
│   ├── file_info.rs     # FileInfo（ファイル詳細情報）
│   ├── file_metadata.rs # VFileMetadata
│   ├── file_time.rs     # VFileTime
│   └── permissions.rs   # VPermissions
└── store/               # 永続化
    ├── mod.rs
    └── bookmark.rs      # BookmarkStore
```

### Module Responsibilities

| Module | Responsibility |
|--------|----------------|
| `config` | アプリケーション設定を保持 |
| `event` | キー入力・ファイル変更を検知し InputEvent として返す |
| `component` | 各エリアのイベント処理・状態・描画を統合（Component trait） |
| `state` | データ型の定義、AppContext（コンポーネントコンテナ） |
| `ui` | レイアウト構成と共通ウィジェット |
| `fs` | ファイルシステム操作 |
| `store` | 永続データの管理（ブックマーク等） |
| `app` | メインループ、Action の処理 |

### Component Pattern

各コンポーネントは `Component` trait を実装します。

```rust
// component/mod.rs
pub trait Component {
    fn handle_event(&mut self, event: KeyEvent) -> Result<Action>;
    fn render(&mut self, frame: &mut Frame, area: Rect) {}
    fn tick(&mut self) {}
}
```

`Action` enum でアプリ全体に影響する操作を表現します。

```rust
pub enum Action {
    None,
    Quit,
    Error(String),
    LaunchShell,
    SetPromptMode(Box<PromptMode>),
    ShowSidePanel(SidePanel),
    // ...
}
```

新しいサイドパネルを追加する場合:
1. `component/` に新しいファイルを作成し `Component` trait を実装
2. `state/side_panel.rs` の `SidePanel` enum にバリアントを追加
3. `SidePanel` の `Component` trait 委譲に match アームを追加
