# Linux 動作確認用 Docker 開発環境を整える

- 作成日: 2026-06-28
- ステータス: ドラフト

## 概要

開発段階で `fv` が Linux 上でもビルド・動作することを確認できるよう、Docker による
開発環境を用意する。Rust ツールチェインと Linux ビルドに必要なシステム依存を揃え、
ホスト（macOS）のソースをマウントしてコンテナ内で `cargo run` / `cargo test` を
実行できるようにする。`fv` は TUI アプリなので、TTY を確保した対話起動も成立させる。

## 背景・前提（コンテキスト）

- `fv` は ratatui ベースの TUI ファイルマネージャ（`docs/CONTEXT.md`）。実行には
  対話的なターミナル（TTY）が必要で、`cargo run` で起動する。
- `Cargo.toml` は edition 2024。これは Rust 1.85 以降を要求するため、コンテナの
  ツールチェインは stable の新しめ（少なくとも 1.85+）である必要がある。
- Linux ビルドに必要なシステム依存は、CI（`.github/workflows/ci.yml` /
  `release.yml`）で確認すると **`libasound2-dev` のみ**。これは `rodio` →
  `alsa-sys` が要求する ALSA 開発ヘッダ。CI は ubuntu-latest 上でこれだけを
  `apt-get install` してビルド・テストを通している。
- CI のツールチェインは `dtolnay/rust-toolchain@stable`。本 Docker 環境もこれに
  合わせ、公式 `rust` イメージ（Debian ベース）の stable を使う。
- 開発依存に含まれる `arboard`（クリップボード）・`rodio`（オーディオ再生）は、
  ヘッドレスなコンテナでは実機能（クリップボード書き込み・音声出力）が働かない
  可能性があるが、**ビルド・起動・基本操作の動作確認**という今回の目的には支障
  しない（CI も追加の X11/Wayland 依存なしでビルドできている）。

## 要件

- Docker で Rust stable + `libasound2-dev` を備えた Linux 環境を構築する。
- ホストのソースをマウントし、コンテナ内で `cargo run` / `cargo build` /
  `cargo test` が実行できる。
- TUI を起動して画面・キー操作を確認できる（TTY 確保、対話モード）。
- コード編集が即コンテナに反映され、再ビルドが速い（`target/` と cargo
  レジストリをキャッシュ）。
- 起動・キャッシュ・TTY 設定は `docker-compose.yml` に集約し、
  `docker compose run --rm fv` 系のワンコマンドで使えるようにする。
- 使い方を README に追記する。

### スコープ外

- 配布用（リリース）イメージの最適化（multi-stage で実行バイナリだけを含む等）。
  今回は開発・動作確認用に限定する。
- Windows コンテナ・クロスコンパイル。
- クリップボード／オーディオなど、ヘッドレス環境で動かない機能の Linux 実機検証。
- CI への Docker 組み込み。

## 確定した論点

- **構成方式 = bind mount + cache**（ユーザー確認済み）。
  - ホストのリポジトリをコンテナの作業ディレクトリに bind mount する。
  - `target/` と cargo レジストリ（`~/.cargo/registry`, `~/.cargo/git`）は
    named volume に逃がし、再ビルドを高速化しつつホストの `target/`（macOS
    向けビルド成果物）と混ざらないようにする。**これは重要**: ホストの
    `target/` をそのままマウントすると macOS と Linux のビルド成果物が衝突する
    ため、コンテナの `target/` は必ず named volume で分離する。
- **成果物 = Dockerfile + docker-compose.yml**（ユーザー確認済み）。
  TTY・ボリューム・作業ディレクトリ設定を compose に集約し、ワンコマンド起動に
  する。
- **ベースイメージ = 公式 `rust:slim`（Debian, stable）**。CI の
  `rust-toolchain@stable` と方針を合わせる。`slim` でイメージを小さく保ちつつ、
  edition 2024 に必要な新しめの stable を得る。

## 実装方針

開発用の単一ステージ Dockerfile を用意する。`rust:slim` をベースに、ビルド依存
（`libasound2-dev` と、`alsa-sys` 等が使う `pkg-config` / `build-essential`）を
`apt-get` で入れる。ソースはイメージに焼き込まず、compose の bind mount で渡す
（編集が即反映、イメージ再ビルド不要）。

`docker-compose.yml` で次を設定する:

- サービス `fv`: 上記イメージをビルド。
- `volumes`: リポジトリルートを作業ディレクトリ（例 `/work`）に bind mount し、
  `/work/target` と cargo レジストリを named volume にする。
- `working_dir: /work`。
- `tty: true` / `stdin_open: true`: TUI 起動に必要な TTY を確保。
- 既定コマンドは `cargo run`。`docker compose run --rm fv` で TUI 起動、
  `docker compose run --rm fv cargo test` 等でコマンド差し替えも可能にする。

README に「Linux での動作確認（Docker）」節を追記し、ビルド・起動・テストの
コマンドを記載する。

## 実装ステップ

1. **Dockerfile を作成**（リポジトリルート `Dockerfile`）。
   - `FROM rust:slim`
   - `apt-get update && apt-get install -y --no-install-recommends
     libasound2-dev pkg-config build-essential` 後にキャッシュ削除。
   - `WORKDIR /work`。
   - 検証: `docker build -t fv-dev .` が成功する。
2. **docker-compose.yml を作成**（リポジトリルート）。
   - サービス `fv`、`build: .`、`working_dir: /work`。
   - bind mount（`.:/work`）+ named volume（`fv-target:/work/target`,
     `fv-cargo-registry:/usr/local/cargo/registry`）。
   - `tty: true`, `stdin_open: true`, `command: cargo run`。
   - 検証: `docker compose run --rm fv cargo build` がコンテナ内で完走する。
3. **TUI 起動を確認**。
   - `docker compose run --rm fv`（既定の `cargo run`）で TUI が描画され、
     基本キー操作（カーソル移動・ディレクトリ移動・終了）ができる。
   - 検証: 画面が表示され、`q` 等で正常終了する。
4. **テスト実行を確認**。
   - `docker compose run --rm fv cargo test` がコンテナ内で通る。
   - 検証: 全テストが Linux 上で green。
5. **README に手順を追記**。
   - 「Linux での動作確認（Docker）」節を追加し、build / run / test の
     コマンドと、target を named volume で分離している旨の注意を記載。
   - 検証: 記載手順どおりに第三者が起動できる。
6. **`.dockerignore` を作成**（任意・推奨）。
   - bind mount 主体なので影響は限定的だが、`docker build` のコンテキスト
     肥大を防ぐため `target/`, `.git/`, `site/` 等を除外。

## 影響範囲・リスク

- 影響を受けるファイル/モジュール:
  - 新規: `Dockerfile`, `docker-compose.yml`, `.dockerignore`
  - 変更: `README.md`（手順追記）
  - ソースコード（`src/`）への変更は不要。
- リスクと対策:
  - **ホスト `target/` との衝突**: macOS と Linux の成果物が混ざるとビルドが
    壊れる。→ コンテナの `target/` を named volume に分離（ステップ 2）。
  - **TUI の TTY 不足**: `docker compose run` ではなく `up` 経由だと TTY が
    付かず TUI が描画されないことがある。→ `run --rm` 利用を README に明記し、
    compose に `tty`/`stdin_open` を設定。
  - **edition 2024 非対応の古い stable**: 古い `rust` イメージだとビルド不可。
    → `rust:slim`（latest stable）を使用。必要なら将来バージョン固定を検討。
  - **クリップボード/オーディオがヘッドレスで無効**: 機能テストには使えない。
    → スコープ外と明記。ビルド・起動・FS 操作の確認に用途を限定。

## 未確定事項

- Rust イメージのバージョン固定の要否（`rust:slim` の latest 追従でよいか、
  `rust:1.XX-slim` に固定するか）。当面は latest stable 追従とし、CI と乖離が
  問題になった時点で固定を検討する。
- `arboard` が Linux で要求する追加依存（X11/xcb 等）。CI は追加なしでビルド
  できているため現状は不要と判断。ビルドエラーが出た場合のみ
  `libxcb1-dev` 等の追加を検討する。
