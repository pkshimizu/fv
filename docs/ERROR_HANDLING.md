# エラーハンドリング方針

本ドキュメントはfvプロジェクトにおけるエラーハンドリングの方針を定めます。

## 方針概要

### 基本方針

- **エラー型**: `anyhow`クレートを使用
- **エラーログ**: `tracing`クレートで`~/.fv/error.log`に出力
- **ユーザー向けメッセージ**: UI層で定義（最大30文字）

### エラー情報の分離

| 対象 | 内容 | 出力先           |
|------|------|---------------|
| 開発者向け | エラーチェーン、スタックトレース、コンテキスト | tracingの設定に依存 |
| ユーザー向け | 簡潔なメッセージ（30文字以内） | UIステータスバー     |

## ライブラリ層（UI層以外）の実装

### 基本ルール

1. 戻り値の型は `anyhow::Result<T>` を使用
2. `.context()` でエラーにコンテキストを追加
3. `?` 演算子でエラー伝播
4. ライブラリ層ではログ出力しない（UI層に委譲）

### 実装例: fs/file.rs

```rust
use anyhow::{Context, Result};
use std::fs::read_dir;
use std::path::Path;

#[derive(Debug)]
pub struct VFile {
    pub path: String,
}

impl VFile {
    pub fn new(path: String) -> Self {
        Self { path }
    }

    /// ディレクトリ内のファイル一覧を取得
    pub fn list(&self) -> Result<Vec<VFile>> {
        let entries = read_dir(&self.path)
            .with_context(|| format!("ディレクトリの読み取りに失敗: {}", self.path))?;

        let mut files = Vec::new();
        for entry in entries {
            let entry = entry.context("エントリの取得に失敗")?;
            let path = entry.path();
            let path_str = path
                .to_str()
                .context("パスの変換に失敗")?
                .to_string();
            files.push(VFile::new(path_str));
        }
        Ok(files)
    }

    /// ファイルサイズを取得
    pub fn file_size(&self) -> Result<u64> {
        let metadata = std::fs::metadata(&self.path)
            .with_context(|| format!("メタデータの取得に失敗: {}", self.path))?;
        Ok(metadata.len())
    }

    /// ディレクトリかどうかを判定
    pub fn is_dir(&self) -> Result<bool> {
        let metadata = std::fs::metadata(&self.path)
            .with_context(|| format!("メタデータの取得に失敗: {}", self.path))?;
        Ok(metadata.is_dir())
    }

    /// ファイル名を取得
    pub fn file_name(&self) -> Result<String> {
        Path::new(&self.path)
            .file_name()
            .context("ファイル名の取得に失敗")?
            .to_str()
            .context("ファイル名の変換に失敗")
            .map(|s| s.to_string())
    }
}
```

## unwrap()/expect() の使用ルール

### 使用を許可するケース

- テストコード
- 起動時の不変条件（ホームディレクトリの取得など）
- 論理的にパニックが適切な場合

### 使用を禁止するケース

- ユーザー操作に依存する処理
- ファイルI/O
- 外部リソースへのアクセス
- ネットワーク操作
