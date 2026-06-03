# 7. Image preview protocol override via environment variable

Date: 2026-06-04
Status: accepted

## Context

画像プレビューの描画プロトコルは起動時に `main.rs` の `Picker::from_query_stdio()` で自動検出している（kitty / sixel / iTerm2 / halfblocks）。しかし ttyd（vhs での画面収録環境）や xterm.js 系の端末は、実際には描画できないグラフィックプロトコルを「対応している」と応答することがある。その結果、自動検出が描画不能なプロトコルを選び、画像プレビューが**空欄**になる（#291）。

検証の結果、こうした端末でも **halfblocks（半ブロック文字）描画なら正しく表示できる**ことが分かった。したがって、自動検出を上書きしてプロトコルを明示指定する手段が必要になった。主な動機は LP のデモ収録のように、自動検出が誤判定する環境で**非対話的に**プロトコルを固定したいケースである。

fv には既に永続設定ストア（`settings.json` + Settings パネル）があるため、素直には「設定項目として追加する」案も考えられる。しかし画像プロトコルは起動時に `Picker` を一度だけ生成して確定するため、セッション途中に設定を変えても効かせるには Picker 再生成の仕組みが要る。また主目的の収録/CI 用途は、起動ごとに「無指定 ↔ 指定」を非対話で切り替えられることが重要で、永続設定とは相性が悪い。

## Decision

- 画像プロトコルの上書きは **環境変数 `FV_IMAGE_PROTOCOL`** で提供する。
- 受け付ける値は `halfblocks` / `sixel` / `kitty` / `iterm2`（大文字小文字無視）。未設定・未知・空文字は無視し、従来どおり自動検出にフォールバックする（警告は出さない）。
- 適用順は「`from_query_stdio()`（失敗時 `halfblocks()`）→ iTerm2 自動判定 → **最後に env 指定を `set_protocol_type()` で適用**」とし、env 指定を常に優先させる。フォントサイズは検出値をそのまま流用する。
- halfblocks 時の注意書き（"Terminal does not support image protocol. Display quality is limited."）は、明示指定の有無にかかわらず**従来どおり表示**する（`image_preview.rs` は変更しない）。
- 値のパースは `main.rs` の純粋関数 `parse_image_protocol(&str) -> Option<ProtocolType>` として実装し、ユニットテストする。

## Considered options

- **永続設定（settings.json + Settings パネル）に追加**: 却下。起動時に Picker を確定するため途中変更を効かせるには再生成機構が必要。収録/CI 用途の非対話・起動ごと切り替えにも不向き。
- **CLI フラグ `--image-protocol`**: 却下。fv に現状コマンドライン引数解析の仕組みが無く、このためだけに導入するのは過剰。
- **環境変数＋永続設定の併用**: 却下。柔軟だが実装・テスト・ドキュメントの負担が最も大きく、現時点の需要に見合わない。
- **halfblocks のみ受け付ける**: 却下。`set_protocol_type` で 4 種すべて対応するコストはほぼ変わらず、「対応端末で検出が外れた」ケースの救済にもなるため全種を受け付ける。

## Consequences

- 環境変数という小さな公開インターフェースが増える。README の「Environment variables」節で値と挙動を記載する。
- env は起動ごとに切り替えられるため、デモ収録・CI・誤検出端末での回避に使える（`FV_IMAGE_PROTOCOL=halfblocks fv`）。
- 自動検出が失敗する端末で halfblocks 以外を強制した場合、フォントサイズが既定値になり描画が粗くなり得る。これはユーザーが明示選択した場合の自己責任とする。
- 将来、画像プロトコルを永続設定や CLI でも指定したくなった場合は、本 ADR の優先順位（env を最優先）を起点に拡張する。
