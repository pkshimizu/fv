---
name: fv
description: fvプロジェクト用のGitHub/Gitワークフローコマンド。GitHub Issueの作成、featureブランチの作成、コミットの作成、プルリクエストの作成、コードレビューを行う。ユーザーが「/fv issue」「/fv branch」「/fv commit」「/fv pr」「/fv review」と入力した場合、またはfvプロジェクトでissue作成、ブランチ作成、コミット、PR作成、レビューを依頼された場合にこのスキルを使用すること。
---

# fv ワークフロースキル

fvプロジェクト用のGitHub/Gitワークフローを提供するスキルです。

## 利用可能なコマンド

| コマンド | 説明 |
|----------|------|
| `/fv issue` | GitHub Issueを作成 |
| `/fv branch` | GitHub Issueに対応するfeatureブランチを作成 |
| `/fv commit` | gitコミットを作成 |
| `/fv pr` | GitHubプルリクエストを作成 |
| `/fv review` | コード変更をレビュー |

## コマンド選択（引数なしの場合）

`/fv` のみで起動された場合（$ARGUMENTSが空の場合）、AskUserQuestionツールを使用して実行するコマンドを選択させる。

AskUserQuestionは最大4つの選択肢しか表示できないため、2つの質問に分ける:

**質問1**: コマンドの種類を選択
- `Git操作` - commit, branch
- `GitHub操作` - issue, pr
- `レビュー` - review

**質問2**: 選択された種類に応じて具体的なコマンドを選択（レビューの場合は不要）

選択後、対応するreferencesファイルを読み込んで処理を実行する。

## コマンド詳細

各コマンドの詳細な手順は `references/` ディレクトリを参照してください。

### /fv issue

GitHub Issueを作成します。ユーザーの要望をINVEST原則に基づいて適切な粒度に分割し、テンプレートを選択して作成します。

**詳細**: `references/issue.md` を読んでください。

### /fv branch [issue_id]

指定されたGitHub Issueに対応するfeatureブランチを作成します。

**詳細**: `references/branch.md` を読んでください。

### /fv commit

現在の変更をコミットします。日本語でコミットメッセージを作成し、適切なprefixを付けます。

**詳細**: `references/commit.md` を読んでください。

### /fv pr

現在のfeatureブランチからプルリクエストを作成します。

**詳細**: `references/pr.md` を読んでください。

### /fv review

現在のブランチのコード変更をレビューします。複数の観点から並列でレビューを実行します。

**詳細**: `references/review.md` を読んでください。

## 共通ルール

- コミットメッセージ、PRタイトル、PR本文は日本語で書く
- Claudeの署名（Co-Authored-Byなど）は付けない
- コマンド実行前に必ず現在の状態を確認する

## Prefix一覧

全コマンドで共通のprefix/分類:

| Prefix | 説明 |
|--------|------|
| `feat` | 新しい機能の実装 |
| `fix` | 不具合修正 |
| `docs` | ドキュメントの変更 |
| `style` | コードフォーマットの修正 |
| `refactor` | リファクタリング |
| `perf` | パフォーマンス・チューニング |
| `test` | テストコードの追加・変更 |
| `chore` | ビルドプロセス、ツール、ライブラリの変更 |
