# キーバインドの追加・変更ルール

キーバインドを追加・変更したら、以下の**すべての面**を同期更新する。1 箇所でも漏れると
ヘルプ・LP・README とアプリの挙動が食い違う。

## 更新箇所

| 面 | ファイル | 内容 |
|----|----------|------|
| 実装 | `src/component/filer.rs`（該当コンポーネント） | `handle_event` のキーハンドラ |
| ヘルプ | `src/component/help.rs` | `KEY_BINDINGS` の該当カテゴリ |
| README | `README.md` | Key bindings 表 |
| LP | `site/index.html` | キーバインド一覧。`data-en` と `data-ja` の**両方**を更新 |
| 用語 | `docs/CONTEXT.md` | キーが用語定義に関わる場合（例: Checked Paths と選択操作） |

## 表記

- 各面はその面の既存スタイルに合わせる（README はバッククォート、LP は `<kbd>` 要素、
  ヘルプは `KEY_BINDINGS` の文字列）。
- 同一面の中では表記を揃える。修飾キー付きは、その面の既存の修飾キー表記に倣う。
- 表示キーと実際の押下を一致させる（例: `Shift`+`A` で発火するなら大文字 `A` を示す）。
  ただしプロジェクト／ユーザーが特定の表記を明示している場合はそれを優先する。

## 未使用キーの選定

- 新しいキーは既存バインドと衝突しない未使用キーから選ぶ。`KEY_BINDINGS` には
  重複キーが無いことを検証するテストがある（`help.rs` の `key_bindings_have_no_duplicate_keys`）。
- 大文字キー（`Shift`+英字）はイベント上 `KeyCode::Char('A')` として届く（`Press` のみ通す）。
