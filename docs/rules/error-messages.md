# エラーメッセージの言語ルール

`fs` / ライブラリ層が `anyhow` で返すユーザー向けエラーメッセージ（`ensure!` / `bail!` /
`with_context` の文言）は、既存に倣い **英語で統一**する。

```rust
// OK: 既存の create 系・rename と同じ英語
anyhow::ensure!(!path.exists(), "{}: File already exists", path.display());
anyhow::bail!("Creating symlinks is not supported on this platform");

// NG: 同一モジュール内の他メッセージが英語なのに日本語を混在させる
anyhow::bail!("このプラットフォームでは未対応です");
```

- 同じファイル・同じ責務の隣接コードと言語を揃える（混在させない）。`fs/file.rs` の
  `create_file` / `create_dir` / `rename` / `create_symlink` は英語メッセージで統一されている。
- UI 層でユーザーに見せる固定文言（ヘルプ・プロンプトのタイトル等）は別途その面の方針に従う。
  ここでの対象は `fs` などライブラリ層が返す `anyhow` メッセージ。
- 新しい `cfg(not(unix))` などのプラットフォーム分岐を足すときも、メッセージ言語を周囲に合わせる。
