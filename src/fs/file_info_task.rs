use crate::fs::VFile;
use crate::fs::file_info::FileInfo;
use anyhow::Result;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::{self, Receiver};

/// `spawn_file_info` の戻り値。`rx` で `FileInfo` 取得結果（Result）を一度だけ受信する。
/// 別ファイル選択などで `FileInfoHandle`（と `rx`）が drop されると、ワーカーの送信は
/// 失敗し結果は破棄される（受信者 drop ＝ キャンセル）。ディレクトリ非同期ロードと同型。
pub struct FileInfoHandle {
    pub rx: Receiver<Result<FileInfo>>,
}

/// ファイル情報取得（`FileInfo::from_file`）を別スレッドで実行し、結果を `rx` に流す。
/// 取得処理は重い（全文読み込み・エンコーディング検出・メディア probe 等）ため UI を
/// ブロックしないよう非同期化する。ワーカーの panic は `catch_unwind` で捕捉し Err 化する。
pub fn spawn_file_info(file: &VFile) -> FileInfoHandle {
    let (tx, rx) = mpsc::channel::<Result<FileInfo>>();
    let file = file.clone();

    let spawn_result = std::thread::Builder::new()
        .name("fv-file-info".into())
        .spawn({
            let tx = tx.clone();
            move || {
                let result = match catch_unwind(AssertUnwindSafe(|| FileInfo::from_file(&file))) {
                    Ok(r) => r,
                    Err(payload) => Err(anyhow::anyhow!(panic_message(payload))),
                };
                // 受信者が既に drop されていれば送信は失敗するが、結果を破棄するだけでよい。
                let _ = tx.send(result);
            }
        });

    // spawn 自体に失敗した場合も「結果は常に rx 経由で届く」という不変条件を保つため、
    // 失敗を載せた結果を同じ channel に流して呼び出し側の通常経路（try_recv）に合流させる。
    if let Err(e) = spawn_result {
        let _ = tx.send(Err(
            anyhow::Error::from(e).context("Failed to spawn file info task")
        ));
    }

    FileInfoHandle { rx }
}

/// `catch_unwind` の panic ペイロードからメッセージ本文を取り出す。
/// 文字列でない場合は原因不明のため固定文言を返す（`app::async_job` と同方針）。
fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    let detail = payload
        .downcast_ref::<&'static str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str));
    match detail {
        Some(s) => format!("file info task panicked: {s}"),
        None => "file info task panicked (non-string payload)".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn spawn_file_info_delivers_file_info_for_a_real_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("note.txt");
        std::fs::write(&path, b"hello\nworld\n").unwrap();
        let file = VFile::new(path.to_str().unwrap());

        let expected = FileInfo::from_file(&file)
            .expect("sync from_file ok")
            .to_lines();

        let handle = spawn_file_info(&file);
        let info = handle
            .rx
            .recv_timeout(Duration::from_secs(5))
            .expect("result received")
            .expect("file info ok");

        assert_eq!(info.to_lines(), expected);
    }

    #[test]
    fn spawn_file_info_delivers_err_for_a_missing_file() {
        let tmp = TempDir::new().unwrap();
        let missing = tmp.path().join("does-not-exist.txt");
        let file = VFile::new(missing.to_str().unwrap());

        let handle = spawn_file_info(&file);
        let result = handle
            .rx
            .recv_timeout(Duration::from_secs(5))
            .expect("result received");

        assert!(result.is_err(), "missing file should yield an error result");
    }
}
