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

### Flux Architecture

本プロジェクトはFluxアーキテクチャに基づいて設計されています。

```
User Input → Event (検知) → Command (Action) → State (Store) → UI (View)
                                                    ↑
                                                 Config
```

### Directory Structure

```
src/
├── main.rs          # エントリーポイント
├── app.rs           # アプリケーションのメインループ
├── config.rs        # 設定関連
├── event.rs         # イベント処理（キー入力の検知）
├── cmd/             # コマンド（Flux の Action）
│   ├── mod.rs
│   ├── command.rs   # Command enum 定義
│   └── quit.rs      # Quit コマンドの実装
├── state/           # 状態管理（Flux の Store）
│   ├── mod.rs
│   └── app.rs       # AppState
├── ui/              # UI描画（Flux の View）
│   ├── mod.rs
│   └── views/
│       ├── mod.rs
│       └── main_view.rs
└── fs/              # ファイルシステム操作
    └── mod.rs
```

### Module Responsibilities

| Module | Responsibility |
|--------|----------------|
| `config` | アプリケーション設定を保持 |
| `event` | キー入力を検知し、Command に変換 |
| `cmd` | アプリケーションで実行可能な操作を定義（enum + 分離パターン） |
| `state` | Command を受け取り状態を更新 |
| `ui` | State に基づいて画面を描画 |
| `fs` | ファイルシステム操作（将来の拡張用） |
| `app` | メインループの制御 |

### Command Pattern

コマンドは enum で定義し、各コマンドの実装は個別ファイルに分離します。

```rust
// cmd/command.rs
pub enum Command {
    Quit,
    None,
}

impl Command {
    pub fn exec(self, state: &mut AppState) {
        match self {
            Command::Quit => quit::exec(state),
            Command::None => {}
        }
    }
}
```

新しいコマンドを追加する場合:
1. `cmd/command.rs` の enum にバリアントを追加
2. `cmd/` に新しいファイルを作成して実装
3. `cmd/mod.rs` でモジュールを宣言
4. `exec` メソッドの match に1行追加
