# Settings パネルを「項目メニュー → 項目エディタ」の2階層フローにする

- 作成日: 2026-06-14
- ステータス: ドラフト

## 概要

`o` で開く Settings パネルは現状「Startup Directory」を直接編集する1階層UI。設定項目が
増える前準備として、**まず変更する項目を選ぶメニュー**を表示し、項目を選ぶと**サイドパネルを
その項目の編集UIに切り替え**、編集を終えると**メニューに戻る**2階層フローへ変更する。
項目追加時は「メニュー項目＋その項目のエディタ」を足すだけで済む構造にする。

## 背景・前提（コンテキスト）

- Settings は `SidePanel::Settings(SettingsComponent)`。Filer の `o` が `Action::ShowSettings`
  を出し、`app.rs` がパネルを生成する。
- 現状の `SettingsComponent` は「Startup Directory」のラジオ（`StartupDirectory::LABELS` の4択）＋
  Specific 選択時のパス入力を直接描画し、`Enter`/`Esc` で `save_or_close()`（dirty なら
  `SaveSettings`、さもなくば `CloseSidePanel`）を返す。
- `app.rs` の `Action::SaveSettings` は **永続化したうえでパネルを閉じている**（`side_panel = None`）。
- 設定項目は現状「Startup Directory」1つのみ。`SaveSettings` を出すのは Settings だけ。
- 並び順 ↔ 値の対応（`LABELS` / `index` / `from_index` / `SPECIFIC_INDEX`）は
  `store/settings.rs` に一元化されている。

## 要件

- Settings パネルを開くと、まず**設定項目のメニュー**（現状 "Startup Directory" の1項目）を表示する。
- メニューで項目を選んで決定すると、サイドパネルを**その項目の編集UI**に切り替える。
- 編集UIで変更を確定するとメニューに戻る。メニューからパネルを閉じられる。
- 既存の「Startup Directory」の編集内容（4択＋Specific のパス）は維持する。
- スコープ外:
  - 新しい設定項目の追加そのもの（本件は前準備。項目は "Startup Directory" のまま）。
  - 各項目エディタの trait 抽象化など、1項目には過剰な一般化。
  - キーバインドの追加（`o` で開く動作は不変）。

## 確定した論点

- **構造**: 単一の `SettingsComponent` に**内部 View 状態（メニュー / 項目エディタ）**を持たせる。
  `SidePanel` は `Settings` のまま・パネルを閉じずに行き来する。項目追加は「メニュー項目＋
  その項目のエディタ状態」を足すだけ。（別コンポーネント＋`SidePanel` バリアント切替は配線が
  増えるため不採用。）
- **保存タイミング**: 項目エディタで **`Enter`＝即保存（`SaveSettings` で永続化）してメニューへ戻る**。
  **`Esc`＝変更を破棄してメニューへ戻る**。**メニューで `Esc` / `o`＝パネルを閉じる**。
  各項目が独立して保存され、状態管理が単純（パネル閉時にまとめて保存する案は dirty 集約が複雑）。
- これに伴い `Action::SaveSettings` は **永続化のみ（パネルを閉じない）** に変更する。閉じるのは
  `CloseSidePanel`（メニューの `Esc`/`o`）に一本化する。`SaveSettings` の発行元は Settings だけなので
  影響は閉じている。

## 実装方針

`SettingsComponent` を2階層化する。

- **View 状態**: `enum View { Menu, Editing }`（または `mode` フィールド）。`render` / `handle_event` /
  `keymap` をこの状態で分岐する。
- **メニュー**: 設定項目のラベル一覧（現状 `["Startup Directory"]`）と選択 index を持つ。
  - `↑`/`↓`: 項目選択を移動。
  - `Enter`: 選択中の項目の編集へ遷移（`View::Editing` にし、エディタ状態を現在値から初期化）。
  - `Esc` / `o`: `CloseSidePanel`。
- **項目エディタ（Startup Directory）**: 既存のラジオ＋パス編集ロジック（`selected_option` /
  `path` / `initial_*` / `is_dirty` / `to_startup_directory`）を**エディタ状態としてまとめる**
  （項目追加時は各項目のエディタ状態を足せる形）。
  - `←`/`→`: ラジオ選択。Specific 選択中は文字キー/`Backspace` でパス編集（既存どおり）。
  - `Enter`: dirty なら `SaveSettings` を返して即永続化し、**コンポーネント内の現在値（baseline）を
    更新**してから `View::Menu` へ戻る（再入時に保存値が出るように）。dirty でなければそのまま
    メニューへ戻る。
  - `Esc`: ドラフトを現在値に戻して（破棄）`View::Menu` へ戻る。
  - `o` の扱い: Specific パス編集中は文字入力として扱い閉じない（既存の `Char('o') if !specific`
    を踏襲）。メニューでのみ `o` を閉じるキーにする。
- **`app.rs`**: `Action::SaveSettings` を「永続化のみ」に変更（`side_panel = None` を削除）。
- **描画/キーマップ**: `render` はメニュー一覧／既存エディタUIを mode で出し分け。`keymap()` も
  mode で出し分け（メニュー: `↑↓: Select  Enter: Edit  o/Esc: Close`、エディタ: 既存の文言）。

## 実装ステップ

1. `app.rs`: `Action::SaveSettings` を永続化のみに変更（パネルを閉じない）。`SaveSettings` の
   発行元が Settings だけであることを確認（grep 済み）。
2. `SettingsComponent` に `View`（Menu/Editing）とメニュー選択 index、現在値 baseline を追加。
   既存のラジオ＋パス状態をエディタ状態として整理する。
3. `handle_event` を mode で分岐:
   - Menu: `↑↓` 選択 / `Enter` で Editing へ / `Esc`・`o` で `CloseSidePanel`。
   - Editing: 既存キー＋ `Enter`（保存して Menu へ）/ `Esc`（破棄して Menu へ）。
4. `render` と `keymap` を mode で分岐（メニュー一覧の描画を追加、エディタ描画は既存流用）。
5. テスト追加（`settings.rs`）:
   - メニューで `Enter` → Editing へ遷移する。
   - Editing で値変更後 `Enter` → `SaveSettings` を返し、Menu へ戻る。
   - Editing で `Esc` → 変更が破棄され Menu へ戻る（`SaveSettings` を返さない）。
   - メニューで `Esc` / `o` → `CloseSidePanel` を返す。
   - 保存後に再度 Editing へ入ると保存値が初期表示される（baseline 更新の回帰）。
6. `cargo fmt` / `cargo clippy --all-targets` / `cargo test` を通す。手動で
   開く→項目選択→編集→保存→メニュー→閉じる、と Esc 破棄の動作を確認する。
7. 必要なら `docs/CONTEXT.md` に「Settings は項目メニュー→項目エディタの2階層」という構造を
   用語/前提として追記する（実装確定時）。

## 影響範囲・リスク

- 影響を受けるファイル/モジュール:
  - `src/component/settings.rs`（中心。2階層化）
  - `src/app.rs`（`SaveSettings` を永続化のみに）
  - `src/state/side_panel.rs`（`Settings` バリアント・keymap 委譲は不変。確認のみ）
  - `docs/CONTEXT.md`（任意。構造の前提を追記する場合）
- リスクと対策:
  - **`SaveSettings` が閉じなくなる挙動変更** → 発行元が Settings のみであることを確認済み。閉じるのは
    `CloseSidePanel` に一本化。
  - **`o` の二役**（メニューで閉じる / Specific 編集中は文字入力）→ mode 分岐と既存の
    `Char('o') if !specific` ガードで誤爆を防ぐ。テストで担保。
  - **保存後の baseline 更新漏れ** → 再入時に古い値が出る。手順5のテストで回帰を防ぐ。
  - **1項目のみのメニューは UX 上やや冗長** → 前準備として意図的（要件）。

## 未確定事項

- 将来項目が増えたときの各エディタの抽象化（trait 化や `SettingItem` enum 化）は、項目が
  実際に増える段階で再検討する（今回は1項目のため最小構成）。
- `docs/CONTEXT.md` への用語追加の要否・文言は実装時に確定する。
