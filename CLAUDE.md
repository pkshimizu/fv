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

Copy / Move / Zip 作成 / Zip 解凍 / Delete の長時間ファイル操作は **Async Job**
（`app/async_job.rs` の worker スレッド + `fs/async_job.rs` の `FileJob`）として実行し、
進捗を `ProgressMessage` で Prompt に流し、Cancel Token で協調的に中断する。

### Directory Structure

```
src/
├── main.rs              # エントリーポイント
├── app.rs               # アプリケーションのメインループ、Action処理
├── app/
│   └── async_job.rs     # Async Job worker（spawn_async_job、進捗スロットリング）
├── app_context.rs       # AppContext（コンポーネントコンテナ）
├── clipboard.rs         # システムクリップボード書き込み（Yank、arboard）
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
│   ├── async_job.rs     # FileJob（Copy/Move/Zip/Delete の二相実行ロジック）
│   ├── file.rs          # VFile（パス・メタデータ・同期的なファイル作成/リネーム）
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
| `fs` | ファイルシステム操作、Async Job の `FileJob` 実行ロジック |
| `clipboard` | システムクリップボードへのパス書き込み（Yank） |
| `store` | 永続データの管理（ブックマーク等） |
| `app` | メインループ、Action の処理、Async Job worker の起動 |

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
    LaunchShell,
    SetPromptMode(Box<PromptMode>),
    ShowSidePanel(Box<SidePanel>),
    // ...
}
```

新しいサイドパネルを追加する場合:
1. `component/` に新しいファイルを作成し `Component` trait を実装
2. `state/side_panel.rs` の `SidePanel` enum にバリアントを追加
3. `SidePanel` の `Component` trait 委譲に match アームを追加

## Agent skills

### Issue tracker

Issues live in this repo's GitHub Issues (via the `gh` CLI). See `docs/agents/issue-tracker.md`.

### Triage labels

Five canonical triage roles mapped to default label strings (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context layout — `CONTEXT.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.
