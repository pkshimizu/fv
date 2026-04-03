---
name: branch
description: GitHub Issueに対応するfeatureブランチを作成する
---

# featureブランチ作成

GitHub Issueに対応するfeatureブランチを作成する。

引数: $ARGUMENTS（GitHub Issue ID）

## 手順

1. 引数からissue IDを取得する。引数が空または数値でない場合はエラーメッセージを表示して終了する
2. `gh issue view {issue_id}` でissueの内容を確認する。issueが存在しない場合はエラーメッセージを表示して終了する
3. issueのタイトルからブランチ名を作成する
   - 形式: `feature/{issue_id}-{summary}`
   - summaryはissueタイトルを英語の短いケバブケース（kebab-case）に変換する
   - 例: issue #3 「ユーザー認証機能の追加」→ `feature/3-add-user-authentication`
4. `git switch -c {ブランチ名}` でブランチを作成して切り替える
5. 作成したブランチ名を表示する

## ルール

- summaryは英語のkebab-caseで、簡潔にすること（3〜5単語程度）
- ブランチ名に使えない文字は除去すること
- mainブランチから作成すること（現在mainにいない場合は `git switch main && git pull` してから作成する）