# 8. Markdown preview via tui-markdown (onig accepted on native builds)

Date: 2026-06-04
Status: accepted

## Context

`v` キーのプレビューで `.md` / `.markdown` をプレーンテキストではなくレンダリング表示したい（#209）。見出し・リスト・コードブロック・強調などをターミナル上で表現する。既存のプレビュー基盤（#129、`PreviewComponent` + `TextOutputState` + `render_text_output`）を拡張する形で実現する。

設計ツリーには複数の分岐があった: レンダリング方式（パーサ自前 vs クレート）、コードブロックのシンタックスハイライトの可否とその依存、スタイル付き行をどの状態に載せるか、コンポーネントを新設するか既存を再利用するか。

特に注意すべき点として、本リポジトリは ADR 0003 で「`alsa-sys` / `arboard` の C・ネイティブ依存リンクが難しい」ことを理由に musl 静的ビルドを避け、**native runner + glibc** でビルドしている。シンタックスハイライトに使う `syntect` は既定で onig（C ライブラリ）に依存するため、この方針との整合を確認する必要があった。

## Decision

- マークダウンのレンダリングは **`tui-markdown` クレート**（内部は `pulldown-cmark`）を用い、ratatui の `Text`（スタイル付き `Line` 群）を得る。
- `tui-markdown` の **`highlight-code` 機能（既定有効、`syntect` によるコードブロックのシンタックスハイライト）はそのまま使う**。`syntect` は onig（C 依存）を引くが、本プロジェクトは native runner + glibc ビルドで既に C 依存（`alsa-sys` / `arboard`）をリンクできているため、**onig でもビルドは壊れない**。コストはバイナリ肥大とコンパイル時間のみで、これを許容する。
- `tui-markdown::from_str` は入力を借用した `Text<'_>` を返すため、サイドパネルに保持できるよう各 `Span` を所有文字列へコピーして **`Vec<Line<'static>>`** に変換する（`ui::markdown::render`）。
- スタイル付き行を扱うため **`TextOutputState` を `Vec<String>` から `Vec<Line<'static>>` に一般化**する。既存の `with_lines(Vec<String>)` は内部で `Line` へ変換して維持し（プレーンテキスト・ヘルプ・ファイル情報の各コンポーネントは無改修）、新たに `with_styled_lines(Vec<Line<'static>>)` を追加する。スクロールの折り返し幅計算は `Line::width()` を使う。
- マークダウンプレビューは **既存 `PreviewComponent` に `new_markdown` を追加**して `SidePanel::Preview` のまま表示する。新しい `SidePanel` バリアントは追加しない。これにより `is_preview()` が既に true で、n/p によるファイル切り替えとプレビューの coalesce（ADR 0006）が自動的に効く。
- 対象拡張子は `.md` / `.markdown`（大文字小文字無視）。判定は `is_markdown_file` を `file_info.rs` に追加し、`build_preview_panel` の分岐（audio → image → markdown → text）に組み込む。
- 読み込み上限・バイナリ判定・truncation は既存の `TextPreview` を流用する（100MB / 1 万行、`(truncated)` 表示）。読み込み失敗は従来どおりメッセージパネルを表示する。

## Considered options

- **`pulldown-cmark` + 自前で ratatui へマッピング**: 却下。UI 非依存で完全に制御できる利点はあるが、基本記法のスタイリングを自前で書く保守負担が大きい。`tui-markdown` が `ratatui-core 0.1`（ratatui 0.30 と統一される）に依存しており型整合も取れるため、クレートに委ねる。
- **シンタックスハイライトを無効化（`default-features = false`）**: 却下しなかったが採用もせず。最軽量だがコードブロックのハイライトが失われる。native ビルドで onig が問題にならないと判明したため、ハイライト有効を選択した。
- **`syntect` を自前統合し純 Rust の `fancy-regex` で onig を回避**: 却下。C 依存を完全に避けられるが、`tui-markdown` を使わず markdown → `Text` 変換を自前実装することになり実装量が最大。native ビルドでは onig が支障にならないため不要と判断。
- **専用 `MarkdownPreviewComponent` + 専用 state + `SidePanel` バリアント追加**: 却下。関心の分離は明確だが、スクロール処理の重複と各 `Component` trait メソッドの match アーム追加が発生する。`TextOutputState` の一般化 + 既存コンポーネント再利用の方が小さく一貫する。

## Consequences

- 依存に `tui-markdown`（および `syntect` / `ansi-to-tui` / onig）が加わり、コンパイル時間とバイナリサイズが増える。クロスビルドはしていないため onig のリンク自体は問題にならない。
- `TextOutputState` がスタイル付き `Line` を保持するようになり、プレビュー・ヘルプ・ファイル情報の全テキスト表示が同じ経路でスタイル付き行を扱える素地ができた（今回挙動は変わらない）。
- マークダウンプレビューは通常のプレビューと同じく n/p で前後ファイルへ切り替えられ、coalesce の対象になる。
- truncation はテキスト同様に行数で打ち切るため、巨大ファイルでは末尾のコードフェンス等が途中で切れてレンダリングが乱れる可能性があるが、上限ケースとして許容する。
- 将来 raw/レンダリング表示の切り替えや、ハイライト無効ビルドのオプション化が必要になれば拡張の余地がある。
