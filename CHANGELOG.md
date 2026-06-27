# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.4](https://github.com/pkshimizu/fv/compare/v0.1.3...v0.1.4) - 2026-06-27

### Other

- Linux 動作確認用の Docker 開発環境を整える

## [0.1.3](https://github.com/pkshimizu/fv/compare/v0.1.2...v0.1.3) - 2026-06-18

### Added

- タブ／複数コンテキストを追加する
- Settings を項目メニュー→項目エディタの2階層フローにする
- コピー/カット/ペースト方式（Paste Buffer）を追加する
- chmod 編集の操作性を改善する
- パーミッション（chmod）を編集できるようにする
- シンボリックリンクを作成できるようにする
- ツリーパネルで f によるファイル名検索を追加する
- 一覧の絞り込みフィルタを追加する
- 全選択／選択クリアのトグルを追加する
- スタートアップディレクトリに任意のディレクトリを指定できるようにする
- e キーで現在ディレクトリをファイルマネージャで開く
- テキストプレビューでシンタックスハイライトを行う
- v キーでマークダウンファイルをレンダリングプレビュー
- 画像プレビューのプロトコルを環境変数で上書き可能にする
- プレビュー中に n/p で選択ファイルを切り替える

### Fixed

- シンボリックリンクを削除できるようにする
- Specific Directory 設定の表示と操作性を改善する
- キーイベントを Press のみ処理し二重処理を防ぐ
- 新規 fv.rb が Homebrew tap に push されない不具合を修正

### Other

- Context 機能のレビュー指摘を反映する
- 履歴テストの独立性を高め、カバレッジを補う
- 戻る/進む履歴を per-context 化する
- prompt.rs の rustfmt 整形漏れを修正する
- Context リスト化のレビュー指摘を反映する
- Filer を単一から Context リストへ内部リファクタする
- Settings のコメントずれを修正する
- Paste Buffer のレビュー指摘を反映する
- chmod 編集の操作性改善にレビュー指摘を反映する
- chmod 編集のレビュー指摘を反映する
- symlink 削除を Unix 限定にしレビュー指摘を反映する
- 一括レビュー指摘を反映する（symlink）
- 一覧検索の巡回ロジックを共通ヘルパへ集約する
- README と LP に絞り込みフィルタの説明を追加する
- 名前フィルタの照合からループ内アロケーションを除く
- 全選択の挿入を HashSet::extend に置き換える
- 開発フローを za プラグインに移行する
- Startup Directory の並び順管理を一元化しレビュー指摘を反映
- README のキーバインド一覧に e（ファイルマネージャで開く）を追記
- ファイル/ディレクトリを開く失敗にcontextを付与
- シンタックスハイライトのレビュー指摘を反映
- RefreshThrottle のレビュー指摘を反映
- システム情報の更新頻度を1秒から5秒に変更する
- マークダウンプレビューのレビュー指摘を反映
- hero-demo.gif の操作キーテロップを削除
- hero-demo.gif に操作キーのテロップを追加
- hero-demo.gif を撮り直し
- LP を Claude Design の新デザインに刷新
- プレビュー coalesce のレビュー指摘を反映
- プレビュー n/p 連打時の再生成を coalesce する
- プレビュー切替のレビュー指摘を反映

## [0.1.2](https://github.com/pkshimizu/fv/compare/v0.1.1...v0.1.2) - 2026-06-02

### Added

- macOS バイナリを Developer ID 署名・公証する

### Other

- README.md を作成する

## [0.1.1](https://github.com/pkshimizu/fv/compare/v0.1.0...v0.1.1) - 2026-06-02

### Added

- ヘルプ表示をカテゴリ別に分けて表示する

### Fixed

- release-plz ジョブに libasound2-dev を追加
- release-plz に git_only を設定しリリースPRが作られない問題を修正

### Other

- release-plz の診断のため verbose とタグ取得を有効化
- ヘルプ生成のレビュー指摘を反映
- 機能セクションのリード文から「（括弧内はキー）」を削除
- LP のモックアップを実スクリーンキャプチャに差し替える
- インストールセクションを特徴と機能の間に移動する
- LP の文言・構成を調整する
- プロジェクトのランディングページを作成する
- Homebrew でインストールできるようにする

## [0.1.0](https://github.com/pkshimizu/fv/releases/tag/v0.1.0) - 2026-06-01

### Added

- ヘッダーに現在時刻（時計）を表示する ([#204](https://github.com/pkshimizu/fv/pull/204))
- ヘッダーにカレントディレクトリの Disk Usage を表示 ([#202](https://github.com/pkshimizu/fv/pull/202))
- システム情報をヘッダーに表示する ([#203](https://github.com/pkshimizu/fv/pull/203))
- y キーで Operation Targets の絶対パスをクリップボードへコピー ([#228](https://github.com/pkshimizu/fv/pull/228))
- ファイル情報取得を非同期化する
- ~キーでホームディレクトリに移動する
- ウィンドウが小さすぎる場合に警告を表示する
- 非同期処理中にアクティビティインジケーター（スピナー）を表示
- サイドパネル表示中のキーマップをプロンプト領域に表示
- Delete の非同期化（trash 経由）
- Zip 作成の非同期化 ([#224](https://github.com/pkshimizu/fv/pull/224))
- Move の非同期化 ([#223](https://github.com/pkshimizu/fv/pull/223))
- Scan Phase + Copy の非同期化 ([#222](https://github.com/pkshimizu/fv/pull/222))
- 非同期ファイル操作の基盤と Zip 展開を実装 ([#221](https://github.com/pkshimizu/fv/pull/221))
- 起動時に前回のカレントディレクトリを復元する設定を追加
- </>キーでディレクトリ履歴の戻る/進む機能を追加
- ディレクトリ遷移履歴の永続化機能を追加
- ?キーでキーバインド一覧のヘルプをサイドパネルに表示する
- xキーで選択ファイルのパスをプロンプトに表示しコマンド実行する
- 画像プロトコル非対応ターミナルで注意メッセージを表示
- vキーで画像ファイルをサイドパネルにプレビュー表示する
- vキーで音声ファイルをサイドパネルで再生する機能を追加
- vキーでテキストファイルの内容をサイドパネルにプレビュー表示する
- ZIP解凍後に解凍先フォルダへカレントディレクトリを変更する
- uキーでZIPファイルを解凍する機能を追加
- tキーでサイドパネルにツリービューを表示する ([#127](https://github.com/pkshimizu/fv/pull/127))
- 非同期ロード中もファイルリストをソート順で維持する
- ディレクトリ走査を非同期化してUIブロックを防ぐ ([#190](https://github.com/pkshimizu/fv/pull/190))
- 非同期処理の進捗表示基盤をpromptエリアに実装する ([#189](https://github.com/pkshimizu/fv/pull/189))
- oキーでアプリ設定画面を表示する機能を追加
- プロンプトのテキスト入力時にカーソル表示とカーソル移動を実装
- pキーで選択ファイルのzip圧縮機能を追加
- nキーで新規ファイル作成機能を追加
- hキーで外部シェルを直接起動するように変更
- jキーでパス入力によるディレクトリ遷移機能を追加
- iキーで選択ファイルの詳細情報をサイドパネルに表示する機能を追加
- シェルコマンド実行機能とサイドパネル表示を実装
- FileプロンプトでもShift+Tabによる候補の逆順送りに対応
- hキーでシェルコマンド入力プロンプトを表示する機能を追加
- aキーで選択中ファイルの属性情報をサイドパネルに表示
- grepコマンドの非同期実行とリアルタイム結果表示を実装
- Grep検索結果の表示パネルとコマンド操作を実装
- gキーでGrep検索プロンプトを表示する機能を追加
- アクティブな領域の枠線色を変更して視覚的に区別
- ブックマーク選択によるディレクトリジャンプと削除の同期
- BookmarkCommandの分離とブックマークパネルのカーソル操作
- ブックマーク一覧パネルの表示切り替え機能を追加
- ブックマーク機能の追加
- ディレクトリ行の文字色を緑色に変更
- ドットファイル行の文字色を青色に変更
- 起動時のカレントディレクトリを初期ディレクトリに設定
- InputMode::None時に簡易キーマップを入力エリアに表示
- 検索モードのキーバインドを変更
- fキーによるファイル検索機能を実装
- sキーでファイルリストのソート順を変更する機能を追加
- InputModeにSelectモードを追加
- .キーでドットファイルの表示・非表示をトグル
- FilerFilterによるドットファイルの表示フィルタリングを実装
- ファイル移動機能を実装
- ファイルコピー機能を実装
- InputMode::Fileを追加しTabキーによるパス補完に対応
- ファイルリストをディレクトリ優先・ファイル名昇順でソート
- エラー発生時にエラーメッセージを入力エリアに表示
- rキーによるファイル名変更機能を追加
- kキーによる新規ディレクトリ作成機能を追加
- 画面下部に入力エリアを追加
- 選択ファイルの一括削除に対応
- Spaceキーによるファイル選択機能を追加
- ファイル削除時にゴミ箱へ移動するように変更
- dキーによるファイル削除機能を追加
- Enterキーでファイルをデフォルトアプリで開く機能を追加
- エラーハンドリングレビューエージェントを追加
- 選択中のファイル行に下線を追加
- /fv起動時のコマンド選択機能を追加
- レビュー用claude code skillを追加
- カレントディレクトリの変更を監視してファイルリストを自動更新
- ファイル一覧にパーミッションカラムを追加
- ファイル一覧に更新日時カラムを追加
- ファイルサイズを3桁区切りで表示
- Backspaceキーによる親ディレクトリへの移動を実装
- Enterキーによるディレクトリ移動を実装
- Fileモデルを追加
- ファイル一覧にサイズ列を追加
- キーボードによるファイル一覧のカーソル移動を実装
- TableStateによるファイル一覧の選択状態管理を追加
- ファイル一覧をTableウィジェットで表示
- FilerStateを追加しカレントディレクトリのパスをステータスバーに表示
- Fluxアーキテクチャに基づくモジュール構造を設計
- ratatuiによるTUIアプリケーションの基本構造を実装

### Fixed

- release-plz がリリースを作成しない問題を修正
- 同一ディレクトリ更新時に一覧をクリアせず差し替えてちらつきを解消
- 親ディレクトリ遷移時に遷移元ディレクトリを選択する
- 起動時の初期ディレクトリにファイル監視を張る
- 単一ファイルの Copy/Move で宛先をファイル名として扱う
- ファイルサイズを単位付きで表示し桁あふれを解消
- dir-symlink を含むディレクトリのコピーが失敗するバグを修正 ([#222](https://github.com/pkshimizu/fv/pull/222))
- ディレクトリ履歴スタックにサイズ上限を追加
- プレビュー非対応ファイルのエラーメッセージを改善
- ライフタイム省略に関するコンパイラ警告を修正
- Kittyターミナルで画像が表示されない問題を修正
- プレビュー非対応ファイルのエラーメッセージを改善
- ZIP解凍後にカレントディレクトリを変更しないようにする
- ツリービューの選択カーソルを > に戻す
- compare_filesにファイル名の二次ソートを追加し同値キー時の順序を安定化
- エラー復元失敗時もユーザーに通知する
- refresh_files時もprev_dirを保持しエラー時に復元可能にする
- 非同期ロードのエラー時に元のディレクトリに復元する
- progress_rxのDisconnected時にクリーンアップを追加
- PromptComponent::tickの二重try_recvをloop+matchに書き換え
- 非同期ロードのエラーをFilerState内で検知しユーザーに通知する
- プロンプトカーソル位置のパディング分のズレを修正
- zip作成エラー時のクリーンアップパスをunique_zip_pathに修正
- レビュー指摘対応（既存ファイル上書き防止・タイトル一貫性）
- BackTabでも初回候補生成を行うように修正
- レビュー指摘対応（文字数カウント修正・infer二重呼び出し排除）
- レビュー指摘対応（kill対応・u16オーバーフロー・クォート・描画最適化）
- レビュー指摘対応（is_file判定・BTreeSet化・Cowアロケーション最適化）
- シェル補完の候補を実行可能ファイルに限定し上限を設定
- レビュー指摘対応（take()安全化・ガード条件修正・clippy警告解消）
- grep selectのエラーメッセージをより正確な表現に修正
- grepのセキュリティ強化とレビュー指摘対応
- grepコマンドのレビュー指摘対応
- grepプロセスのspawnをメインスレッドに移動しエラーを伝播
- grep非同期処理のレビュー指摘対応
- grep.rsのカーソル操作がbookmarkの状態を操作していたバグを修正
- エラー表示のボーダー色をBorderStyle::Errorに変更
- get_paths()の不要な.clone()を除去
- レビュー指摘の軽微な修正
- ブックマーク削除のバグ修正とインデックス補正
- ブックマーク保存をアトミック書き込みに変更
- add()の冗長なhasチェック除去とセミコロン追加
- マルチバイト文字での検索誤マッチを修正
- ファイル名変更後にカーソル位置がリセットされる問題を修正
- is_apply_for_dirsの戻り値をメソッド名の意味に合わせて修正
- ソート変更時にチェック状態がクリアされる問題を修正
- サイズソート時にディレクトリを名前昇順で並べるよう修正
- toggle_show_dot_fileでカーソル位置とチェック状態を維持するよう修正
- move_toのフォールバックをEXDEVエラーに限定し宛先パスの二重解決を解消
- move_toにクロスデバイス移動のフォールバックを追加
- renameでparent()がNoneの場合にエラーを返すよう修正
- compute_path_candidatesをVFile::list()のAPIに合わせて修正
- unique_pathのループに上限を設定し整数オーバーフローを防止
- file_nameの戻り値型の構文エラーを修正
- 複数ファイル操作時にエラー件数を正しく報告するよう改善
- copy_dir_recursiveでシンボリックリンクの循環参照を防止
- unique_pathの戻り値をResult<PathBuf>に変更しunreachableを除去
- アクション実行後のchecked_pathsクリアとコピーエラーメッセージ改善
- renameで同名ファイルが存在する場合にエラーを返すように修正
- VFile::renameにwith_contextを追加
- create_dirにパストラバーサル対策と空文字列チェックを追加
- create_dirの所有権ムーブによるコンパイルエラーを修正
- レビュー指摘事項の修正（with_context・unreachable・String::new）
- 入力エリアの確認表示のtypoを修正
- 未使用インポートの削除とtrash::deleteにエラーコンテキストを追加
- レビュー指摘事項の修正（symlink_metadata・エラー優先順位・表示修正）
- レビュー指摘事項の修正（エラー保持・シンボリックリンク対応・簡略化）
- レビュー指摘事項の修正（エラー収集・Default・コメント追加）
- レビュー指摘事項の修正
- VFileTimeの変数名utc_timeをlocal_timeに修正
- watch_directoryのエラー握りつぶしを修正
- prev/firstにも空Vec時のガードを追加
- FilerStateのnext/lastで空Vec時のunderflowを防止
- レビュー指摘に基づくエラーハンドリング修正
- ディレクトリ移動時にカーソル位置を先頭にリセット

### Other

- 廃止された macos-13 ランナー対応として macOS x86_64 ビルドを打ち切る
- release-plz で GitHub Release ベースのリリースフローを構築
- クロスプラットフォームビルドのCI/CDをGitHub Actionsで構築
- Disk Usage のレビュー指摘を反映 ([#202](https://github.com/pkshimizu/fv/pull/202))
- border のレビュー指摘を反映 ([#257](https://github.com/pkshimizu/fv/pull/257))
- ビュー border をフォーカス=線種・状態=色の2軸に再設計 ([#257](https://github.com/pkshimizu/fv/pull/257))
- ADR 0002 プレビュー翻訳・DeepL プロバイダ抽象を追加
- Translation Request のキャンセル仕様を受信者 drop に修正
- システム情報のレビュー指摘を反映 ([#203](https://github.com/pkshimizu/fv/pull/203))
- clipboard を src/os/ 配下へ移動 ([#203](https://github.com/pkshimizu/fv/pull/203))
- 分割のレビュー指摘を反映 ([#251](https://github.com/pkshimizu/fv/pull/251))
- async_job.rs を操作ごとのモジュールに分割 ([#251](https://github.com/pkshimizu/fv/pull/251))
- ループ統一のレビュー指摘を反映 ([#249](https://github.com/pkshimizu/fv/pull/249))
- Async Job の Operation Phase ループを共通ヘルパに統一 ([#249](https://github.com/pkshimizu/fv/pull/249))
- yank のレビュー指摘を反映 ([#228](https://github.com/pkshimizu/fv/pull/228))
- ファイル情報非同期化のレビュー指摘を反映 ([#196](https://github.com/pkshimizu/fv/pull/196))
- change_to_home の中間 String 確保を避け &str を直接渡す
- レビュー指摘に基づき receive_files の整理とカーソル復元の共通化
- レビュー指摘に基づき警告描画の堅牢性と可読性を改善
- レビュー指摘に基づきテストの可読性と復元側コメントを改善
- レビュー指摘に基づき監視失敗を degrade しエラー context を付与
- レビュー指摘に基づき宛先判定を一度だけ評価する形に整理
- レビュー指摘に基づきスピナーの整形を集約し Default を実装
- レビュー指摘に基づきキーマップの体裁規約と可視性を整理
- Merge pull request #235 from pkshimizu/feature/file-size-display
- レビュー指摘に基づきサイズ整形の責務分離と命名を改善
- 削除済み fs::file ヘルパを指す stale コメントを修正 ([#226](https://github.com/pkshimizu/fv/pull/226))
- 非同期化後の dead_code・未使用ヘルパを撤去 ([#226](https://github.com/pkshimizu/fv/pull/226))
- 2回目レビュー指摘に基づき Delete 実装を改善 ([#225](https://github.com/pkshimizu/fv/pull/225))
- rustfmt によるフォーマット調整
- コードレビュー指摘に基づき Delete 実装を改善 ([#225](https://github.com/pkshimizu/fv/pull/225))
- rustfmt によるフォーマット調整
- コードレビュー指摘に基づき Zip 作成を改善 ([#224](https://github.com/pkshimizu/fv/pull/224))
- rustfmt によるフォーマット調整
- コードレビュー指摘に基づき Move 実装を改善 ([#223](https://github.com/pkshimizu/fv/pull/223))
- 2 回目のレビュー指摘を反映 ([#222](https://github.com/pkshimizu/fv/pull/222))
- CONTEXT.md に Selection 用語を追加
- コードレビュー指摘に基づき非同期基盤を改善
- エージェント向けスキル用のドキュメントを整備
- 非同期ファイル操作の用語集とADRを追加
- forward履歴クリアをtruncateに置き換え
- ディレクトリ履歴のback/forwardを永続化されたHistoryStoreで管理する
- レビュー指摘に基づくHistoryStoreの改善
- キーバインド定義の相互参照コメントを追加
- 2回目のレビュー指摘に基づく改善
- レビュー指摘に基づくコマンド実行機能の改善
- 画像サイズチェックのcollapsible_if警告修正とオーバーフロー対策
- レビュー指摘に基づく画像プレビューの改善
- AUDIO_EXTSの配列リテラルのフォーマットを修正
- 2回目のレビュー指摘に基づく改善
- レビュー指摘に基づく音声再生機能の改善
- ツリービューのアイコンを変更
- ツリービューのフォーマットを整理
- マージ処理のmatch式のフォーマットを整理
- receive_filesをバッチマージ方式に変更しO(n²)をO(k log k + n)に改善
- エラー復元処理のフォーマットを整理
- フィルタ条件をis_visible_nameメソッドに一元化
- prev_dirの保存をmem::replaceに変更しクローンを回避
- progress_rxのComplete受信時にクリーンアップを追加
- 不要なProgressMessage::Update送信を削除
- receive_filesのprogress_rx消費でUpdate/Completeを意図的に無視する旨を明記
- countとPROGRESS_NOTIFY_INTERVALの型をusizeに統一
- start_progressの将来利用意図をコメントで明示
- checked_paths.retainの計算量をO(n*m)からO(n+m)に改善
- 進捗通知間隔のマジックナンバーを定数化
- フィルタロジックの同期/非同期パス間の同期を促すコメントを追加
- receive_filesのループをwhile条件に変更
- FilerStateに手動Debug実装を追加
- promptへの進捗表示を廃止しfilerタイトルのLoading表示に統一
- レビュー指摘対応（Disconnectedエラー通知・tick制御フロー改善・ProgressMessage移動）
- レビュー指摘対応（ProgressMessage移動・借用パターン改善）
- レビュー指摘対応（dirty判定・フィールド可視性統一）
- StartupDirOptionを廃止しStartupDirectoryにlabel/ALL/indexを統合
- CLAUDE.mdのディレクトリ構造をAppContext移動に追従
- AppContextをsrc/直下に移動し変数名をctxに統一
- 旧アーキテクチャのクリーンアップ（カプセル化強化）
- cmd/廃止・AppContext改名・render空実装解消・CLAUDE.md更新
- レビュー指摘対応（bookmark統一・InputEvent分離・委譲メソッド追加）
- FilerをComponentアーキテクチャに移行
- レビュー指摘対応（イベント処理統合・Tab共通化・パターンマッチ簡素化）
- PromptをComponentアーキテクチャに移行
- レビュー指摘対応（SidePanelにComponent実装・tick導入・as_component削除）
- Bookmark/GrepパネルをComponentアーキテクチャに移行
- レビュー指摘対応（is_component一元化・clone除去・網羅性チェック）
- レビュー指摘対応（借用分離・render集約・dead_code限定・リネーム）
- Attribute/FileInfoパネルをComponentアーキテクチャに移行
- Component traitとAction enumの定義、App基盤の整備
- zip圧縮のファイル名重複回避を実行時に移動
- filer.rsのPath参照をuse宣言で統一
- レビュー指摘対応（zip I/Oロジックをfs層に移動・堅牢性向上）
- フォーマッタによるコード整形
- レビュー指摘対応（resume順序・デッドコード削除・バリアント位置）
- レビュー指摘対応（候補計算関数の分離・Jumpバリデーション追加）
- ファイル情報の共通項目（パーミッション・日時）を上部に移動
- サイズフォーマットをVFileMetadata::formatted_size()に統合
- show_attributeのガードスタイルをshow_file_infoと統一
- レビュー指摘対応（symlink判定・ラベル&str化・マジックナンバー定数化）
- build_text_outputのライフタイムを借用に変更しString cloneを除去
- テキスト出力の描画を表示範囲のみに限定し毎フレームの全行クローンを排除
- total_visual_linesをキャッシュ化しスクロール時のO(n)走査を排除
- compute_shell_candidatesのエラーハンドリング意図をコメントで明記
- compute_shell_candidatesのHashSetとcloneを排除しsort+dedupに置換
- FileとShellのキーマッピングをorパターンで統合
- execute_shell_actionでShellActionをmatchして将来のバリアント追加漏れを防止
- compute_shell_candidatesの不要な変数バインディングを除去
- cycle_candidatesにCycleDirectionを導入しTab/BackTabのロジックを統合
- unreachable!()にガード意図を示すメッセージを追加
- render_main_viewのレイアウト構築とfiler描画の重複を解消
- select関数のメソッドチェーンをrustfmtに合わせて改行
- show系関数に既存サイドパネルの上書き防止ガードを追加
- app.rsのuse文をネストimportに統一
- select関数でOption::take()を使い所有権移動を明示
- hide/select関数にサイドパネルのバリアントチェックを追加
- サイドパネルの状態管理をSidePanel enumに統合
- AttributeState::newでmetadataの不要なcloneを除去
- entriesをnew時にキャッシュしmetadataフィールドとrow_countを廃止
- 属性エントリ構築をAttributeStateに集約しrow_countの二重管理を解消
- レビュー指摘対応（エラー伝播・エリア判定順序・コード整形）
- Vecと文字列の事前容量確保で不要な再アロケーションを回避
- VPermissionsのDRY化とFluxアーキテクチャ準拠の改善
- 属性テーブルの行数管理を動的化しVPermissionsのカプセル化を強化
- Unix固有APIを#[cfg(unix)]で条件コンパイルに変更
- 属性テーブルの日時表示順序とファイルテーブルのカラム幅を調整
- grep結果受信ロジックをPathListState::receive_resultsに移動
- grep結果受信にフレームあたりの上限を追加
- build_path_tableで中間Vec<Row>の生成を排除
- build_bookmark_tableとbuild_grep_tableを共通のbuild_path_tableに統合
- BookmarkStateとGrepStateを共通のPathListStateに統合
- レビュー指摘に基づく追加修正
- レビュー指摘に基づくgrep機能の軽微な修正
- レビュー指摘の軽微な修正
- BorderStyle enumの導入とレビュー指摘対応
- TableCursorアダプタでカーソル操作ロジックを共通化
- BookmarkStoreのpathsフィールドを非公開化
- InputModeをPromptModeにリネーム
- レビュー観点の30文字制限削除とUI表示文字の定数化
- Executableトレイトを廃止しCommandラッパーenumによる静的ディスパッチに変更
- レビュー最終指摘の対応
- BookmarkStoreのnew()からファイルI/Oを除去
- BookmarkStoreをBTreeSetに変更し最終レビュー指摘対応
- BookmarkStoreの細かな改善
- BookmarkStoreのjson_pathをコンストラクタで確定
- BookmarkStoreのAPI改善と不要なderive除去
- BookmarkStoreのレビュー指摘対応
- InputAreaCommandをPromptCommandにリネーム
- Executableトレイトによるコマンドのポリモーフィズム導入
- FilerCommandの入力モード開始系バリアントにPromptプレフィクスを付与
- CommandをFilerCommandとInputAreaCommandに分割
- file_tableのis_dir()呼び出しを変数に集約
- ドットファイルスタイルの定数化とmut排除
- 検索機能の軽微な改善
- レビュー指摘に基づくファイル検索機能の改善
- ファイル検索機能の重複ロジックを共通化
- SortKeyのcompareメソッドの重複コードを解消
- is_size()をis_not_apply_for_dirs()にリネーム
- Selectモードの選択項目に反転表示を追加
- reload_current_dirを廃止しrefresh_filesに統一
- レビュー指摘に対応
- clippy提案に従いmap_orをis_none_orに変更
- FilerFilterのレビュー指摘を対応
- VFile::removeメソッドを追加しmove_toのフォールバック処理を簡潔化
- レビュー指摘3件を対応
- レビュー指摘3件を対応
- unique_pathのマジックナンバーを名前付き定数に変更
- VFile::file_name()の戻り値をOption<&str>に変更しアロケーション削減
- execute_*_actionの冗長な?;Ok(())パターンを除去
- checked_paths.clear()をinput_okに集約しexecute_*_actionから除去
- VFile::is_dir()の戻り値をResult<bool>からboolに変更
- clippy警告の修正とコードスタイル改善
- compute_path_candidatesの戻り値をResultに変更しエラー伝播を改善
- execute_copy/execute_deletesを最初のエラーで即時中断に変更
- clippy指摘とunique_pathのエラーハンドリング改善
- レビューエージェントの出力に箇所情報と該当コードを追加
- ソート処理のis_dirをunwrap_or(false)で明示化
- ソート処理をmatchタプルパターンに書き換え
- ファイルリスト読み込みをload_current_dirに集約
- InputDeleteConfirmをInputDeleteにリネーム
- InputActionをTextActionとConfirmActionに分離
- fv-issue skillの確認をAskUserQuestionの選択式に変更
- InputAction enumを導入しInputModeから業務データを分離
- 入力エリアのパディングをPadding::horizontalに変更
- OpenDeleteConfirmをInputDeleteConfirmにリネーム
- input_okの削除ロジックをexecute_deletesに分離
- 未使用のInputMode::Confirmバリアントを削除
- input_okのワイルドカードを明示的なバリアント列挙に変更
- ファイル削除確認をモーダルから入力エリアに変更
- fv-issue skillのユーザー確認を選択式に変更
- open_delete_modalでcurrent_dir_filesから検索するように変更
- checked_pathsの除去処理をretainに簡略化
- レビュー指摘事項の修正（retain→remove・公開範囲・iter）
- レビュー指摘事項の修正（HashSet化・未使用import削除・to_string整理）
- deleteメソッドのエラーコンテキスト追加と不要な参照の除去
- レビュー指摘事項の最終修正
- コードフォーマットの修正
- レビュー指摘事項の修正（clippy準拠・網羅性・Eq追加）
- modal_confirmでstd::mem::replaceを使用して所有権を取り出す
- DeleteConfirmに削除対象ファイルリストを保持するように変更
- 削除モーダルのレイアウトとファイル削除処理を改善
- file.rsのanyhow::Resultインポートスタイルを統一
- app.rsのanyhow::Resultインポートスタイルを統一
- cmd内のファイル構成を操作単位からリソース単位に変更
- VFileにis_dirメソッドを追加しFS詳細のカプセル化を改善
- metadataエラーにファイルパスのコンテキストを追加
- VFile::newをimpl Into<String>に変更し不要なto_string()を除去
- absolute_pathの戻り値をStringから&strに変更
- change_toメソッドの冗長なto_string除去と親ディレクトリ移動の統合
- enter_fileの借用問題を解消しchange_dir_in_select_dirをchange_toに整理
- main_viewの変数名filter_areaをfiler_areaに修正
- VFileTimeのto_stringをDisplayトレイト実装に置き換え
- change_dir_in_select_dirでis_dir判定をlist()の前に移動
- VFile::new()のメタデータ取得を簡潔に記述
- VFileのメタデータをコンストラクタで取得しキャッシュ
- スキルのディレクトリ名とnameフィールドにfv-プレフィックスを追加
- 各スキルのdescriptionを簡潔な記述に統一
- skills/fvの統合スキルを5つの独立スキルに分離
- list()の不要な中間Vec collectを除去
- VFileMetadata構造体によるmetadata取得の一括化
- VFileのmetadata取得をプライベートメソッドに集約
- list_size()の毎フレームI/Oを排除
- エラーハンドリング方針から.context()の要件を削除
- VFileメソッドのエラーハンドリングを方針に準拠
- anyhow::Resultへの統一とunwrap除去
- VFileのエラーハンドリングを改善
- エラーハンドリング方針を策定
- ファイル時刻の処理をVFileTime構造体に分離
- VPermissionsの未使用メソッドに警告抑制を追加
- カスタムスラッシュコマンドをskillsに移行
- FilerCursorをMoveCursorにリネーム
- FilerStateでVFileモデルを使用するように変更
- FilerStateでファイル一覧を直接保持するように変更
- AppStateにquitメソッドを追加
- UIコンポーネントをfeaturesモジュールに分離
- CLAUDE.mdにFluxアーキテクチャの設計を追記
- importパスを明示的に修正
- PRコマンドからCopilotレビュアー設定を削除
- ratatuiとcrosstermの依存関係を追加
- PRコマンドにCopilotレビュアー設定を追加
- PRコマンドの対応Issue記載形式にfixesプレフィックスを追加
- Copilot code reviewのカスタム指示ファイルを追加
- PRコマンドの対応Issue記載形式を箇条書きに変更
- Claude Code の導入とGitHubテンプレートの追加
- create project
