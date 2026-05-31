# 4. Release flow: release-plz with single-workflow build chaining via GITHUB_TOKEN

Date: 2026-06-01
Status: accepted

## Context

main へのマージのたびにビルド/リリースするのではなく、**GitHub Release の作成をリリースの起点**としたい。あわせてリリースごとにアプリのバージョン（`Cargo.toml`）を更新し、可能なら自動化したい（issue #262）。

二つの制約がこの設計を駆動する:

1. **バージョンはバイナリに埋め込まれる。** `fv` はヘッダーで `env!("CARGO_PKG_VERSION")` を表示する（`src/ui/features/header.rs`）。これはコンパイル時に `Cargo.toml` の値が焼き込まれるため、リリースバイナリは **bump 済みの `Cargo.toml` からビルド**しなければ表示が食い違う。したがってバージョンの真実の源はタグではなく `Cargo.toml` 側であり、bump → ビルドの順序が必須。
2. **コミットは Conventional Commits 準拠。** fv-commit が `feat:` / `fix:` / `refactor:` などの prefix を強制しており、コミット種別から semver の bump 量を機械的に判定できる。

GitHub には「`GITHUB_TOKEN` が作成したタグ/Release は他ワークフローを連鎖起動しない（無限ループ防止）」という仕様があり、リリース作成からバイナリビルドへ繋ぐ経路の設計が必要になる。

## Decision

- **release-plz を採用。** main への push で「リリース PR」（`Cargo.toml` の semver bump + `CHANGELOG.md` 生成）を自動作成/更新し、その PR のマージを**リリースの意思決定点**とする。マージ後 release-plz がタグと GitHub Release を作成する。Conventional Commits からの自動 bump が制約 2 に合致し、PR マージという明示操作が「毎マージ自動リリースにしない」要件を満たす。
- **crates.io へは publish しない（設定は `release-plz.toml` 側）。** `fv` はライブラリではなくバイナリアプリ。`release-plz.toml` に `publish = false` を設定して `cargo publish` だけをスキップし、タグ・GitHub Release・CHANGELOG は通常通り作成する。`CARGO_REGISTRY_TOKEN` は不要。**`Cargo.toml` の `[package] publish = false` は使わない** — release-plz はそれをパッケージ自体の除外と解釈し、リリース PR もリリースも一切作らなくなるため（この罠で初回リリースが作成されなかった）。
- **リリースの起点はリリース PR のマージ（`release_always = false`）。** 既定の `release_always = true` だと、まだ未リリースの `Cargo.toml` 版が push 時に即リリースされ、PR を介さない。明示的なリリース操作（PR マージ）を起点とするため false にする。
- **認証はデフォルトの `GITHUB_TOKEN` のみ。PAT / GitHub App は使わない。** cascading trigger 制約は、**同一ワークフロー内の job 依存**で回避する。`release.yml` 内で release-plz ジョブの出力 `releases_created` を見て、`needs` + `if` でビルドジョブを起動する。連鎖起動に頼らないため、シークレット管理ゼロ・期限なし。
- **`build.yml` を `release.yml` へ置換。** 旧 `build.yml`（main push のたびに 3 プラットフォームをビルドしアーティファクト化）を廃止。リリース作成時のみ macOS arm64 / Linux x86_64 を release ビルドし、`gh release upload` で Release 資産に添付する。
- **macOS x86_64（Intel）ターゲットを打ち切る。** ADR 0003 では `macos-13` ランナーでネイティブ Intel ビルドを行う設計だったが、`macos-13` ランナーは 2025-12-04 に GitHub で retired された。移行先は `macos-15-intel`（2027-08 まで提供）だが、Intel Mac 向けビルドを維持し続ける価値が薄いと判断し、対象を **macOS arm64 / Linux x86_64 の 2 ターゲット**に縮小する。ADR 0003 の macOS x86_64 に関する判断は本 ADR が supersede する。
- **リリースタグは `Cargo.toml` のバージョンから導出。** 単一クレートのため release-plz のタグは `v<version>` 形式。ビルドジョブはマージ後 main（bump 済み）をチェックアウトしており、`Cargo.toml` の version からタグ名を組み立てる。これは埋め込みバージョンと必ず一致し、action の出力 JSON スキーマに依存しない。
- **`ci.yml` に PR 時のクロスプラットフォーム build チェックを追加。** アーティファクトは作らず、3 ターゲットの `cargo build` が通ることだけを検証。リリース時まで macOS/Linux のビルド破綻に気づけない事態を防ぐ。

## Considered options

- **GitHub App / PAT トークン:** 却下。リリース PR で CI が走り、リリースイベントの連鎖でワークフロー分割も可能になる利点はあるが、App は初期セットアップが重く、PAT は期限・個人紐付き・広い権限の難点がある。個人プロジェクトではシークレット管理を避けたく、単一ワークフロー連結で同等の自動化が得られる。代償はリリース PR が `ci.yml` を自動起動しない点だが、PR の中身は `Cargo.toml` と `CHANGELOG.md` のみで影響は小さい。
- **macOS x86_64 を `macos-15-intel` で継続 / arm64 からクロスコンパイル:** 却下。前者は 2027-08 までの期限付きで延命にすぎず、後者は実行検証ができない（ADR 0003 で却下済みの理由）。Intel Mac の需要が低いため target ごと打ち切る方を選んだ。
- **手動タグ push / workflow_dispatch でのバージョン選択:** 却下。動作するが、Conventional Commits から自動 bump できる利点を捨てることになる。
- **cargo-dist (dist):** 却下。インストーラ生成やチェックサムまで包括するが、個人のバイナリ TUI には過剰でセットアップが重い。
- **タグを真実の源にする純タグ駆動フロー:** 却下。制約 1（バージョン埋め込み）により、`Cargo.toml` を放置したままタグだけ進めるとアプリ表示が古い版のままになる。

## Consequences

- リリース PR は `GITHUB_TOKEN` で作成されるため `ci.yml` を自動起動しない。中身が限定的なので許容するが、CI を確実に回したい場合は将来 App/PAT への切り替えが必要。
- release-plz が GitHub Release を作成した直後はバイナリ未添付の状態が数分続き、ビルドジョブ完了時に資産が揃う。許容範囲。
- 資産は `fv-<target triple>` 名の生バイナリ。利用者は実行前にリネーム/`chmod +x` が必要。配布物としてアーカイブ化やインストーラが必要になれば再検討する。
- ワークスペース化（複数クレート化）した場合、release-plz のタグ形式が `<package>-v<version>` に変わり、`release.yml` のタグ導出を見直す必要がある。
