GitHubのプルリクエストを作成してください。

## 手順

1. `git branch --show-current` で現在のブランチ名を確認する
2. ブランチ名が `feature/*` パターンでない場合、エラーメッセージを表示して終了する
3. ブランチ名から issue ID を取得する
   - `feature/{issue_id}-{title}` の形式から issue ID（数値）を抽出する
   - 取得できなくても処理は継続する
4. `git log main..HEAD --oneline` でコミット履歴を確認する
5. `git diff main...HEAD` で変更差分を確認する
6. `.github/pull_request_template.md` のテンプレートに沿ってPR本文を作成する
7. リモートにブランチをpushする（未pushの場合）
8. `gh pr create` でプルリクエストを作成する

## PRテンプレートの埋め方

- **概要**: コミット履歴と差分から変更内容を要約（1〜3文）
- **対応Issue**: issue IDが取得できた場合は `- https://github.com/{owner}/{repo}/issues/{issue_id}` のように箇条書きでURLを記載。取得できない場合は「対応issueなし」と記載
- **変更内容**: コミット履歴と差分から箇条書きで記載
- **テスト**: 差分からテストコードの追加・変更内容を確認し箇条書きで記載。テストコードの変更がない場合は「なし」と記載

## ルール

- PRタイトルとPR本文は日本語で書くこと
- baseブランチは `main` を使用すること
- PRタイトルはコミット履歴から簡潔にまとめること
- issue IDがある場合、PRタイトルの先頭に `#{issue_id}` を付けること
- Claudeの署名は付けないこと
- PR本文はHEREDOCで渡すこと
- 作成後、PRのURLを表示すること