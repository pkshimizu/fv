# 3. Cross-platform CI builds: glibc + native runners, split CI/build workflows

Date: 2026-06-01
Status: accepted（macOS x86_64 ターゲットと `build.yml` に関する判断は ADR 0004 が supersede）

## Context

リリースに向けて macOS（arm64 / x86_64）と Linux（x86_64）向けバイナリを GitHub Actions で自動ビルドする必要がある（issue #160）。設計ツリーには複数の分岐があった: ワークフローの分割方針、Linux の libc（glibc vs musl）、macOS x86_64 の作り方（ネイティブ runner vs クロスコンパイル）、Linux で導入すべきシステムライブラリ。

特に Linux のシステム依存は「文脈なしだと驚く」点を含む。本プロジェクトは音声再生に `rodio`（→ `cpal` → `alsa-sys`）、クリップボードに `arboard` を使う。`arboard` は `default-features = false` で構成されており、Linux バックエンドは純 Rust の `x11rb`（XCB）に解決され、wayland バックエンド（`wl-clipboard-rs`）は無効。したがって `Cargo.lock` には `alsa-sys` は現れるが X11 系の `-sys` クレートは現れない。

## Decision

- **ワークフローを 2 分割。** `ci.yml`（`pull_request` + `push: main`）で fmt --check / clippy -D warnings / test を実行。`build.yml`（`push: main`、手動 `workflow_dispatch` も可）でクロスプラットフォームの release ビルド + アーティファクトアップロード。PR は軽量・高速に保ち、重い 3 プラットフォームビルドは main に集約する。
- **Linux は glibc（`x86_64-unknown-linux-gnu`）。** musl 静的ビルドは採用しない。`alsa-sys` と `arboard` のネイティブ依存が musl 環境でのリンクを難しくし、ビルドが不安定になるため。glibc + `ubuntu-latest` のネイティブビルドが確実。
- **macOS は 2 ターゲットをネイティブ runner で分けてビルド。** arm64 は `macos-14`（`aarch64-apple-darwin`）、x86_64 は `macos-13`（`x86_64-apple-darwin`）。クロスコンパイルではなくネイティブにすることで、各アーキで実テスト・実行検証が可能。
- **Linux のシステム依存は `libasound2-dev` のみ。** `rodio` → `alsa-sys` が ALSA の開発ヘッダ/pkg-config ファイルを要求するため apt で導入。`arboard` は `x11rb`（純 Rust）に解決されるため X11 系開発ライブラリは不要。`ci.yml` の `cargo test` もバイナリクレートをリンクするので、ci/build 双方で `libasound2-dev` を入れる。
- **アーティファクトは release ビルドした生バイナリを `fv-<target triple>` 名でアップロード。** `actions/upload-artifact` が自動で zip 化するため、追加圧縮はしない。

## Considered options

- **1 ワークフロー統合（PR でも全ビルド）/ issue 文面通り push:main のみ:** 却下。前者は PR ごとに 3 プラットフォームビルドが走り重い。後者はマージ前にチェックが効かない。2 分割が PR の速度とビルド集約を両立する。
- **Linux musl（`x86_64-unknown-linux-musl`）:** 却下。単体バイナリの可搬性は魅力だが、`alsa-sys` / `arboard` の C・ネイティブ依存リンクが困難で CI が不安定になる。
- **macOS x86_64 を arm64 runner からクロスコンパイル:** 却下。runner 1 台で済むが x86_64 バイナリの実行検証ができず、ネイティブ build の確実性を優先。
- **X11 系 apt パッケージ（libxcb-\*-dev 等）の導入:** 不要として却下。`arboard` の Linux バックエンドは `x11rb`（純 Rust XCB）で、システム X11 開発ライブラリを必要としない。`Cargo.lock` に X11 系 `-sys` クレートが無いことで裏付けられる。

## Consequences

- 依存構成が変わって新たなネイティブ依存（例: wayland バックエンド有効化、別の `-sys` クレート追加）が入った場合、Linux の apt 依存を見直す必要がある。現状は `libasound2-dev` のみ。
- アーティファクトは生バイナリ。リリース配布物として tar.gz 等で固めたい場合は将来 `build.yml` にアーカイブ手順を足す（または GitHub Releases 連携の別 issue とする）。
- macOS universal binary（lipo 統合）は作らない。arm64 / x86_64 を別アーティファクトとして提供する。
