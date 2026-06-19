# Homebrew 6.0.0 Tap Trust に対応したインストール手順の更新

- 作成日: 2026-06-19
- ステータス: ドラフト

## 概要

`brew install fv` が `Error: Refusing to load formula pkshimizu/tap/fv from untrusted tap pkshimizu/tap` で
失敗する。原因は **Homebrew 6.0.0（2026-06-11 リリース）で導入された Tap Trust** で、サードパーティ
tap は明示的に信頼するまで formula を評価しなくなったため。fv 側の formula やリリースパイプラインの
不具合ではなく、すべてのサードパーティ tap に等しく起きる仕様変更。対応は**インストール手順（README・
LP）に `brew trust` ステップを追記する**こと。

## 背景・前提（コンテキスト）

- 配布は `pkshimizu/homebrew-tap`（別リポジトリ）の `fv.rb` 経由。本リポジトリでは
  `.github/homebrew/fv.rb.tmpl` をリリースワークフロー（`.github/workflows/release.yml` の
  `update-homebrew` job）でレンダリングし、tap リポジトリへ push している。
- 現在のインストール手順は 2 箇所:
  - `README.md` の「Homebrew」節（`brew tap pkshimizu/tap` → `brew install fv`）。
  - `site/index.html` の install カード（`<pre>` の表示と、コピー用 `data-copy` 属性の両方）。
- formula テンプレート（`fv.rb.tmpl`）は素直な `url` + `sha256` + `bin.install "fv"` のみで、
  Tap Trust を回避できる要素は無い（そもそも回避策は存在しない）。

### 調査で判明した点（Homebrew 6.0.0 Tap Trust）

- 6.0.0 以降、**公式 Homebrew tap と組み込みコマンドのみが既定で信頼**される。サードパーティ tap・
  tap 修飾された formula/cask・外部コマンドは、コードを評価/実行する前に明示的な信頼が必要。
  理由はサードパーティ tap が任意の非サンドボックス Ruby を実行しうるため（セキュリティ強化）。
- **メンテナ側が「既定で信頼される」状態にする方法は無い**（公式 tap、または API/公式リモート由来の
  コンテンツのみ自動許可）。したがって fv 側の設定変更では解消できず、利用者が `brew trust` する
  運用が前提になる。
- 公式ドキュメントの推奨:
  - 反復利用: `brew tap user/repo` → `brew trust --formula user/repo/formula` → `brew install formula`
    （`brew trust` は tap 済みであることが前提）。
  - 一回限り: `brew install user/repo/formula`（その項目だけ信頼。tap 不要）。
  - 粒度は「必要な formula だけ信頼」を推奨。tap 全体の信頼は将来の formula も含め信頼してよいときのみ。
- 出典: Homebrew 6.0.0 リリースノート（https://brew.sh/2026/06/11/homebrew-6-0-0/）、
  Tap Trust ドキュメント（https://docs.brew.sh/Tap-Trust）、Homebrew/brew Discussion #6876・Issue #22551。

## 要件

- README・LP の Homebrew インストール手順を、Tap Trust 下で**そのまま成功する**手順に更新する。
- 主推奨は **formula 単位トラスト**の 3 ステップ（ユーザー確定事項）:
  ```
  brew tap pkshimizu/tap
  brew trust --formula pkshimizu/tap/fv
  brew install fv
  ```
- なぜ trust が必要かを 1 行で補足する（Homebrew 6.0.0 のセキュリティ仕様であること）。
- スコープ外:
  - formula テンプレート（`fv.rb.tmpl`）・リリースワークフローの変更（Tap Trust は formula 側で
    回避できないため不要）。
  - tap 全体トラスト（`brew trust pkshimizu/tap`）を主推奨にすること（採用せず。補足として触れる程度に留める）。
  - Homebrew 以外の配布（GitHub Releases 手順）の変更。

## 確定した論点

- **原因**: Homebrew 6.0.0 の Tap Trust。fv の bug ではなく仕様変更。メンテナ側の自動信頼化は不可
  （調査で確定）。→ 対応はドキュメント更新に限定。
- **主推奨コマンド**: formula 単位トラスト（`brew trust --formula pkshimizu/tap/fv`）の 3 ステップ。
  Homebrew 公式の「必要な formula だけ信頼」推奨に沿い、最小権限で安全（ユーザー選択で確定）。
- **更新範囲**: 本リポジトリの `README.md` と `site/index.html`（`data-en`/`data-ja` と `data-copy` の
  両方）。加えて、別リポジトリ `pkshimizu/homebrew-tap` の README にも同じ手順への更新が必要な旨を
  本プランに明記する（実作業は別リポジトリのため本タスク外。ユーザー選択で確定）。

## 実装方針

ドキュメントのみの変更。コード・CI・formula には触れない。

- **README.md「Homebrew」節**: コードブロックを 3 ステップ（tap → trust --formula → install）に更新し、
  直前に「Homebrew 6.0.0 以降、サードパーティ tap は `brew trust` が必要」である旨を 1 行で補足。
- **site/index.html の install カード**:
  - 表示用 `<pre>` を 3 行（tap / trust --formula / install）に更新。
  - コピー用 `data-copy` 属性を同じ 3 コマンド（改行は `&#10;`）に更新（表示とコピーの二重管理に注意）。
  - 補足文（`data-en`/`data-ja`）に trust が必要な旨を必要なら追記。`data-en` と `data-ja` の両方を
    更新する（keybindings ルールと同じく LP は両言語同期）。
- **tap リポジトリ（別リポジトリ・本タスク外）**: `pkshimizu/homebrew-tap` の README にも同じ
  3 ステップ手順を反映する必要がある旨を、本プラン「未確定事項／関連作業」に記録。

## 実装ステップ

1. `README.md` の Homebrew 節を 3 ステップ手順＋補足 1 行に更新する。
   → 完了確認: 記載コマンドが Tap Trust 下で成功する順序（tap → trust → install）になっている。
2. `site/index.html` の install カードの `<pre>` と `data-copy` を 3 ステップに更新し、補足文の
   `data-en`/`data-ja` を整える。
   → 完了確認: 表示・コピー・両言語が一致。`data-copy` の改行エンコード（`&#10;`）が正しい。
3. 文言の最終確認（コマンドのタイポ、formula 修飾名 `pkshimizu/tap/fv` の正しさ）。
   → 完了確認: README と LP で手順・表記が一致。
4. 別リポジトリ `pkshimizu/homebrew-tap` の README 更新を別タスクとして起票/共有する。

## 影響範囲・リスク

- 影響を受けるファイル: `README.md`、`site/index.html`（いずれもドキュメント）。コード・テスト・CI への
  影響なし（`cargo` 系の検証は不要だが、LP は GitHub Pages にデプロイされるため表示崩れに注意）。
- リスクと対策:
  - **表示とコピーの不一致**（`<pre>` と `data-copy` の二重管理）→ ステップ 2 で両方を必ず更新し目視確認。
  - **手順の順序ミス**（trust より先に install すると同じエラー）→ tap → trust → install の順を厳守。
  - **tap リポジトリ README の取り残し** → ステップ 4 で別タスク化して追跡。
  - **将来 Homebrew の trust 仕様が変わる**可能性 → 出典 URL をプランに残し、再確認できるようにする。

## 未確定事項／関連作業

- 別リポジトリ `pkshimizu/homebrew-tap` の README 更新（同じ 3 ステップ手順）。本リポジトリの
  za フローでは扱えないため、別途対応する。
- 一回限り派の `brew install pkshimizu/tap/fv`（tap 不要・対話プロンプト想定）を「補足」として併記
  するかは、実装時に README/LP の分量を見て判断（必須ではない）。
