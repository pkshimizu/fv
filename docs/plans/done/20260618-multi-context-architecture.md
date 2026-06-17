# タブ／複数 Context のアーキテクチャ

- 作成日: 2026-06-18
- ステータス: ドラフト

## 概要

単一 Filer 前提のアプリを、複数の独立した作業状態（**Context**）を保持して切り替えられる
構造へ拡張する。各 Context は独立したカレントディレクトリ・カーソル・Checked Paths 等を
持ち、`Tab`/`Shift+Tab` で巡回、`w` で新規作成、`Shift+W` でクローズする。複数ディレクトリを
行き来しながらの作業（Context 間の move/paste 等）を楽にするのが狙い。本プランは issue #305
の resolve-issue コメントで確定した方針を前提に、**状態モデルとアーキテクチャの設計**を詰める
（issue #305 が「着手前に個別に詰める」よう推奨していた部分）。

## 背景・前提（コンテキスト）

確定済みの方針（issue #305 の resolve-issue コメント / 承認済み）:

- **用語**: 複数作業場所の呼称は **Context**（CONTEXT.md に用語追加）。UI 上のタブ表示は
  Context の視覚表現。
- **キー操作**: `Tab`=次の Context / `Shift+Tab`=前 / `w`=新規 / `Shift+W`=クローズ。
  Filer フォーカス時のみ。
- **per-context 状態**: カレントディレクトリ・カーソル位置・Checked Paths・sort・List Filter・
  dotfile 表示・**戻る/進む履歴** を Context ごとに独立。
- **global 状態**: Paste Buffer・Bookmark・Side Panel・Prompt・Settings・System Info/Disk/Clock。
- **生成/上限/クローズ**: 新規=現在 Context のカレントディレクトリを複製 / 個数上限なし /
  最後の1つはクローズ不可。
- **UI**: header と content の間に 1 行のタブバー（番号 + dir 名、アクティブ強調）。

調査で判明した現状（実装の起点）:

- `AppContext`（`src/app_context.rs`）は単一の `filer: FilerComponent` を持つ。`prompt` /
  `side_panel` / `system_info` / `disk_usage` / `paste_buffer` も保持。
- `FilerComponent`（`src/component/filer.rs`）は `state: FilerState` / `focused: bool` /
  `picker: Picker` / `spinner: Spinner` を持つ。per-context にしたい状態は実質 `FilerState`
  （`current_dir` / `current_dir_files` / `file_table_state` / `checked_paths` / `sort_key` /
  `filter` / 読み込み用 rx 等）に集約されている。
- `Picker`（`ratatui_image::picker::Picker`、ratatui-image 11.0.2）は `#[derive(Clone, Debug)]`。
  font size・protocol type 程度の小さな値型なので**クローンは安価**。Context ごとに複製できる。
- 履歴は `store/history.rs` の `HistoryStore` が **back/forward（`cursor`）と永続化（`add` /
  `last_entry` / `history.json`）の両方**を担う。`<`/`>` は `Action::NavigateBack/Forward` →
  `store.history.back()/forward()`。Startup の Last Directory（`SettingsStore` の
  `StartupDirectory::LastDirectory`）は `history.last_entry()` を参照する。
- メインループ（`src/app.rs` `run`）は `self.ctx.filer.current_dir_path()` を監視し、変化したら
  `event_handler.watch_directory` の張り替えと `store.history.add` を行う（`skip_history_add` で
  履歴・進む/戻る時の二重追加を抑止）。
- `ctx.filer` への参照は `app.rs`・`prompt.rs`・`ui/features/header.rs`・`ui/views/main_view.rs`
  に計 25 箇所程度。キー入力は run ループで prompt → side_panel → filer の優先順で振り分け、
  side panel か prompt がアクティブな間は filer はキーを受け取らない（`set_focused` も同様）。
  → **Side Panel / Prompt が開いている間は Filer キー（Tab/w/W）が届かない**ので、Context 切替は
  自然と「サイドパネル・プロンプトが閉じているとき」だけに限定される。

## 要件

- 複数の Context を保持し、`Tab`/`Shift+Tab` で巡回切替できる。
- `w` で現在 Context のカレントディレクトリを複製した新規 Context を作成し、それをアクティブにする。
- `Shift+W` で現在 Context をクローズできる（Context が 1 つのときは不可＝サイレント no-op）。
- per-context 状態（dir / カーソル / Checked Paths / sort / List Filter / dotfile / 戻る進む履歴）が
  Context ごとに独立する。
- Paste Buffer は global で、ある Context で `Ctrl+C`/`X` → 別 Context へ切替 → `Ctrl+V` できる。
- header と content の間にタブバーを表示し、各 Context（番号 + dir 名）とアクティブが分かる。
- `help.rs` / LP(`site/index.html`) / `README.md` / `docs/CONTEXT.md` にキーと用語を反映する。
- スコープ外:
  - Context のセッション跨ぎ永続化（開いていた Context 群の復元）。今回はセッション内のみ。
  - Context のドラッグ並べ替え・リネーム・複製先ディレクトリ指定などの高度な操作。
  - 数字キー（`1`-`9`）による直接切替（resolve-issue で Tab 巡回案を採用済み。将来拡張）。
  - 分割ペイン（同時に複数 Context を並べて表示）。あくまで 1 画面 1 アクティブ Context。

## 確定した論点

- **Context の表現**: `AppContext` に Context のリストとアクティブ index を持たせる。
  各 Context は `FilerComponent` ＋ per-context のディレクトリ履歴をまとめた構造体
  `Context`（仮）とする。理由: per-context 状態は実質 `FilerComponent`（中の `FilerState`）に
  集約済みで、Context 単位で丸ごと持つのが最小変更かつ責務が明快。別 Component＋`SidePanel`
  的な配線は不要（Context は Filer の多重化であってサイドパネルではない）。
- **Picker の扱い**: 新規 Context 作成時に `Picker` を**クローン**して各 `FilerComponent` に渡す
  （`Picker: Clone` で安価）。`Rc` 共有や `AppContext` への巻き上げは、得られる利得が小さく
  変更面が広がるため採らない。
- **履歴の分割（resolve-issue の保留点を確定）**:
  - セッション内の `<`/`>`（戻る/進む）は **per-context のインメモリ履歴** `DirHistory`（仮）で
    持つ。`HistoryStore` の `cursor`/`back`/`forward` のロジックを永続化から切り離してこの型へ移す。
  - 永続化（`history.json`）と Startup の Last Directory は従来どおり **global** の `HistoryStore`
    が担い、**アクティブ Context のディレクトリ移動**を `add` で反映する（`last_entry` も従来どおり）。
  - 理由: 戻る/進むは「その作業文脈」での操作なので per-context が自然。一方で「前回終了時の
    場所」は 1 つで十分なので、アクティブ Context 基準の global 永続でよい。これにより
    `StartupDirectory::LastDirectory` の挙動は現状維持できる。
- **Context 切替時の永続履歴**: 切替・新規作成で `skip_history_add` を立て、切替先ディレクトリを
  `history.json` に重複追記しない（その dir は初回訪問時に追記済み）。理由: 切替は「移動」では
  ないため、永続履歴・Last Directory を切替操作で汚さない。
- **Side Panel / Prompt と Context 切替の関係**: Side Panel か Prompt がアクティブな間は Filer に
  キーが届かないため、Context 切替・新規・クローズは**それらが閉じているときのみ**発生する。
  したがって「Context をまたいだ stale なプレビュー」は構造的に発生しない。global な Side Panel /
  Prompt の所有のままでよい（Context ごとに持たない）。
- **Async Job と Context**: Async Job は Prompt（global）が所有し、起動時にアクティブ Context の
  カレントディレクトリを dest として確定する。実行中は Filer Lock で入力が Prompt に占有され
  Context 切替できないため、dest がずれる余地はない。Job 完了後の `refresh_files` はアクティブ
  Context に対して行う（起動時とアクティブが同一であることが保証される）。
- **キー入力の流路**: `Tab`/`Shift+Tab`/`w`/`Shift+W` は **アクティブ Context の
  `FilerComponent::handle_event` で受け、新 Action（`NextContext` / `PrevContext` /
  `NewContext` / `CloseContext`）を返す**。`AppContext`（`app.rs` の `handle_action`）が
  リスト・アクティブ index を更新する。既存の Action パターン（`ShowBookmark` 等）に一致。
  - `Shift+W` は `KeyCode::Char('W')`、`Tab` は `KeyCode::Tab`、`Shift+Tab` は `KeyCode::BackTab`。
    いずれも現行 filer のキーと未衝突（`w`/`z`/`Tab`/`BackTab` は空き。`A` のみ既存使用）。
- **タブバー UI**: header(3 行) と content の間に高さ 1 のバーを追加。各 Context を
  `1:name 2:name …` 形式（name はディレクトリ末尾要素を短縮）で並べ、アクティブを反転表示。
  幅が足りないときはアクティブ近傍を優先して省略（`…`）。最小ウィンドウ判定
  (`meets_minimum_size`) と高さレイアウトを 1 行ぶん調整する。

## 実装方針

段階的に、各ステップで `cargo build`/`test` が通る状態を保ちながら進める。中心は
「単一 `filer` → Context リスト」への置換で、まず**振る舞いを変えずに 1 Context へ内部リファクタ**
してから、切替・UI・履歴分割を足す。

### データ構造（新規・変更）

- `src/state/context.rs`（新規）: per-context 履歴 `DirHistory`（`entries: Vec<String>` ＋
  `cursor`、`back`/`forward`/`push` を `HistoryStore` の現ロジックから移植・非永続化）と、
  Context をまとめる構造体（`FilerComponent` ＋ `DirHistory`）。命名は CONTEXT.md の用語
  「Context」に合わせる（型名は `Context` 衝突回避のため `FilerContext` 等を検討）。
- `src/app_context.rs`: `filer: FilerComponent` を `contexts: Vec<FilerContext>` ＋
  `active: usize` に置換。`active_filer(&self)` / `active_filer_mut(&mut self)` /
  `active_history_mut` 等のアクセサを追加。`tick` は全 Context をまわす（非アクティブも
  ディレクトリ読み込みやスピナーを進める必要があるか確認の上、最低限アクティブ＋読込中を tick）。
- `src/component/mod.rs`: `Action` に `NextContext` / `PrevContext` / `NewContext` /
  `CloseContext` を追加。
- `src/store/history.rs`: back/forward/cursor を `DirHistory` 側へ移し、`HistoryStore` は
  永続化（`add` / `last_entry` / `load` / `save`）に専念させる。

### 制御フロー

- `src/component/filer.rs`: `handle_event` の冒頭付近で `KeyCode::Tab`/`BackTab`/`Char('w')`/
  `Char('W')` を新 Action にマップ。
- `src/app.rs`:
  - `ctx.filer` の参照を `ctx.active_filer()/active_filer_mut()` に全面置換（約 25 箇所）。
  - `handle_action` に新 Action を追加: index 巡回、`NewContext`（アクティブの dir を複製した
    `FilerContext` を push してアクティブ化、`Picker` クローン、`skip_history_add` を立てる）、
    `CloseContext`（`contexts.len() > 1` のときのみ remove、active を補正）。
  - `NavigateBack/Forward` をアクティブ Context の `DirHistory` 経由に変更。
  - `run` ループのディレクトリ監視・`history.add` をアクティブ Context 基準に。切替/新規時は
    `skip_history_add` で永続追記を抑止しつつ、`watch_directory` の張り替えと `refresh` は行う。
- `src/ui/views/main_view.rs`: レイアウトに高さ 1 のタブバー領域を追加し、`render_tab_bar`
  を呼ぶ。`meets_minimum_size` / 高さ定数を 1 行ぶん見直す。
- `src/ui/features/`（新規 `tab_bar.rs` 等）: タブバー描画。`header.rs` の現在ディレクトリ参照を
  アクティブ Context に。

### ドキュメント同期（keybindings ルール）

- `src/component/help.rs` `KEY_BINDINGS`: 新カテゴリ（例「Contexts」）に Tab/Shift+Tab/w/W を追記。
  `key_bindings_have_no_duplicate_keys` テストを通す。
- `README.md` / `site/index.html`(`data-en`/`data-ja` 両方) のキー表に追記。
- `docs/CONTEXT.md`: 「Context」用語を追加（per-context / global 状態の線引き、Tab/w/W、最後の 1 つは
  閉じない、Paste Buffer は global で Context 間 paste 可能、という要点）。CLAUDE.md の
  「Single-context layout」表現の更新要否も確認。

## 実装ステップ

1. **内部リファクタ（振る舞い不変）**: `AppContext` を `contexts: Vec<FilerContext>`＋`active`
   （要素数 1）に置換し、`active_filer*` アクセサを導入。`app.rs`/`prompt.rs`/`ui` の `ctx.filer`
   参照をアクセサへ置換。`FilerContext` は当面 `FilerComponent` のみラップ（履歴は次ステップ）。
   → 完了確認: 既存テスト・手動操作が従来どおり（単一 Context として全機能動作）。
2. **履歴の分割**: `DirHistory` を新設し back/forward を per-context 化。`HistoryStore` を永続化
   専念に。`NavigateBack/Forward` と run ループの `add` を新構成へ。
   → 完了確認: `<`/`>` が（単一 Context でも）従来どおり動き、`history.json`・Startup Last
   Directory が変わらない。`DirHistory`/`HistoryStore` の単体テスト追加。
3. **Context 操作 Action**: `Action` に 4 種追加、filer のキーマップ追加、`handle_action` 実装
   （巡回・新規=dir 複製＋Picker クローン・クローズ=最後は不可）。切替時の `watch_directory`
   張替え・`refresh`・`skip_history_add`。
   → 完了確認: `w`/`Tab`/`Shift+Tab`/`Shift+W` で Context が増減・巡回し、各 Context の dir/カーソル/
   Checked Paths/sort/filter/履歴が独立。Paste Buffer で Context 間 paste ができる。`handle_action`
   の Context 操作にユニットテスト。
4. **タブバー UI**: レイアウトにバーを追加し描画。最小サイズ・高さ定数を調整。header をアクティブ
   Context 参照に。
   → 完了確認: タブバーに Context 一覧とアクティブが表示され、狭幅でも崩れない。`main_view` の
   レイアウト/警告テストを更新。
5. **ドキュメント同期**: help/README/LP/CONTEXT.md を更新。
   → 完了確認: `key_bindings_have_no_duplicate_keys` パス、4 面の表記一致。
6. **総仕上げ**: `cargo fmt` / `cargo clippy --all-targets`（警告なし）/ `cargo test`。手動で
   新規→切替→各状態の独立→Context 間 paste→クローズ（最後は不可）→`<`/`>` の per-context 性、
   Async Job 中は切替不可、を確認。

## 影響範囲・リスク

- 影響を受けるファイル/モジュール:
  - 新規: `src/state/context.rs`（`FilerContext` / `DirHistory`）、`src/ui/features/tab_bar.rs`。
  - 変更（中心）: `src/app_context.rs`、`src/app.rs`、`src/component/mod.rs`(Action)、
    `src/component/filer.rs`(キーマップ)、`src/store/history.rs`（back/forward 移設）、
    `src/ui/views/main_view.rs`、`src/ui/features/header.rs`、`src/component/prompt.rs`
    （`ctx.filer`→アクセサ）、`src/component/help.rs`、`README.md`、`site/index.html`、
    `docs/CONTEXT.md`。
- リスクと対策:
  - **`ctx.filer` 全面置換の波及（約 25 箇所）** → ステップ 1 で振る舞い不変のまま機械的に置換し、
    その時点でテスト・手動確認。以降の機能追加と分離する。
  - **履歴分割で Startup Last Directory が壊れる** → `HistoryStore.add`/`last_entry` は維持し、
    アクティブ Context のナビゲーションのみ供給。ステップ 2 で history.json と Last Directory の
    回帰をテストで担保。
  - **非アクティブ Context のディレクトリ読み込み（async rx）の進行** → `tick`/受信処理を全 Context
    に行き渡らせないと、切替時に未完了読み込みが固まる恐れ。`AppContext::tick` と run ループの
    受信ドレインを「アクティブ＋読み込み中の Context」に対して回す設計にする（要検証）。
  - **タブバー追加で content 高さが 1 行減る／最小サイズ判定** → `meets_minimum_size` と高さ定数を
    見直し、`main_view` テストを更新。
  - **Async Job 実行中の切替** → Filer Lock により入力が Prompt 占有のため発生しない（確認済み）。
    念のため Context 操作 Action 側でも Job 実行中はガードするか検討。

## 未確定事項

- `FilerContext` / `DirHistory` の正確な型名・配置（`state/` か新規 `context/`）は実装時に確定。
  CONTEXT.md 用語「Context」との衝突を避ける命名にする。
- 非アクティブ Context の読み込み・`tick` をどこまで進めるか（全件 tick か、アクティブ＋読込中のみ
  か）は、ディレクトリ読み込みの実装（`dir_load_rx` の所在）を見て決める。ステップ 1〜3 で確定。
- タブ名の短縮規則（末尾要素のみ / 文字数上限 / 同名衝突時の区別）は UI 実装（ステップ 4）で詰める。
- CLAUDE.md の「Single-context layout」表記を更新するか、CONTEXT.md 用語追加に留めるかは実装時判断。
