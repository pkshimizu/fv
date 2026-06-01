# 5. Homebrew distribution: tar.gz assets + release workflow pushes formula to the tap

Date: 2026-06-02
Status: accepted

## Context

macOS ユーザーが `brew install` で fv を導入できるようにしたい（issue #162）。配布先の tap リポジトリ `pkshimizu/homebrew-tap` は既存で、別ツール（`co` / `do`）の Formula が GoReleaser 生成で置かれている（tar.gz 配布、`on_macos`/`on_linux` × `on_intel`/`on_arm` 構成）。fv は Rust 製で GoReleaser を使わず、リリースは release-plz + `release.yml`（ADR 0004）で行う。

二つの前提が設計を縛る:

1. **リリース成果物の形式。** ADR 0004 では生バイナリ（`fv-<target>`）を Release 資産にしていた。Homebrew は素のバイナリ URL も扱えるが、ダウンロード後のファイル名解決が面倒で、慣習・既存 tap の流儀ともにアーカイブが前提。
2. **tap は別リポジトリ。** fv の `release.yml` から `homebrew-tap` へ書き込むには、リポジトリ内に閉じたデフォルト `GITHUB_TOKEN` では不足で、クロスリポジトリの権限が要る。

## Decision

- **Release 成果物を tar.gz 化する。** 各ターゲットのバイナリを中身 `fv` の `fv-<target triple>.tar.gz` に固めて Release に添付する。Homebrew の `bin.install "fv"` が素直に書け、既存 `co`/`do` の流儀とも揃う。ADR 0004 の「生バイナリ配布」はこの点を本 ADR が supersede する。
- **対象プラットフォームは macOS arm64 / Linux x86_64 の 2 つ。** ビルドしているのがこの 2 つのため、Formula も `on_macos`→`on_arm` と `on_linux`→`on_intel`（64bit）だけを定義する。Intel Mac / Linux arm64 は非サポート（`brew install` はエラー）。ソースビルドのフォールバックは入れない（Formula を単純に保つ）。
- **Formula は `release.yml` の `update-homebrew` ジョブが生成・push する。** build ジョブ後に動き、Release の tar.gz をダウンロードして sha256 を算出し、`.github/homebrew/fv.rb.tmpl` のプレースホルダ（バージョン・各 sha256）を埋めて `fv.rb` を生成、`homebrew-tap` に commit/push する。GoReleaser がやることを自前ジョブで再現する形。
- **クロスリポジトリ認証は fine-grained PAT。** `homebrew-tap` のみに `contents: write` を絞った PAT を fv リポジトリの Secret `HOMEBREW_TAP_TOKEN` として登録し、push に使う。デフォルト `GITHUB_TOKEN` は他リポジトリへ書けないため。
- **crates.io（`cargo install`）は今回スコープ外。** ADR 0004 で `release-plz.toml` の `publish = false` とした方針と矛盾するため、Homebrew のみとする。

## Considered options

- **生バイナリのまま Formula 側で吸収:** 却下。`download_strategy` や `bin.install` のファイル名指定が煩雑で、既存 tap の流儀とも外れる。
- **ソースビルドのフォールバックを Formula に入れて全 PF 網羅:** 却下。`depends_on "rust" => :build` で Intel Mac 等もカバーできるが、Formula が複雑化しユーザーに Rust ビルドを強いる。需要が読めない段階では 2 PF 限定で十分。
- **macOS x86_64 を `macos-15-intel` で復活:** 却下。ADR 0004 で打ち切った判断を踏襲。Homebrew のためだけに期限付きランナーを復活させる価値は薄い。
- **`dawidd6/action-homebrew-bump-formula` 等の既製アクション:** 却下。2 プラットフォーム限定・独自構成の Formula とは相性が読みにくく、テンプレート生成を自前で持つ方が制御しやすい。
- **クロスリポジトリ認証に GitHub App / deploy key:** 却下。App は初期セットアップが重く、deploy key は SSH 鍵管理が要る。tap 限定の fine-grained PAT が最小権限・最小手間。

## Consequences

- **Homebrew が使えるのは次回リリース以降。** 既存の v0.1.0 は生バイナリで Formula も無いため、tar.gz と `fv.rb` が揃う次のリリースから `brew install` が機能する。
- **手動の前提作業が 1 つある。** リリース前に、`homebrew-tap` に `contents: write` を絞った fine-grained PAT を作成し、fv リポジトリに Secret `HOMEBREW_TAP_TOKEN` を登録しておく必要がある。未登録だと `update-homebrew` ジョブが失敗する。
- **Formula の真実の源はテンプレート。** `fv.rb` はワークフロー生成物（tap 側で手編集しない）。Formula の体裁を変えたいときは `.github/homebrew/fv.rb.tmpl` を編集する。
- Linux arm64 や Intel Mac の需要が出たら、ビルド対象の追加（ADR 0004 の見直し）と Formula のブロック追加が必要になる。
