use anyhow::{Context, Result, bail};
use std::io::{BufRead, BufReader, Read, Seek};

const MAX_PREVIEW_LINES: usize = 10_000;
const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100MB
const TEXT_DETECT_BUF_SIZE: usize = 8192;

pub struct TextPreview {
    pub lines: Vec<String>,
    pub truncated: bool,
}

impl TextPreview {
    pub fn from_file(path: &str) -> Result<Self> {
        let mut file =
            std::fs::File::open(path).with_context(|| format!("Failed to open {path}"))?;
        // ファイルサイズ制限
        let file_size = file
            .metadata()
            .with_context(|| format!("Failed to get metadata {path}"))?
            .len();
        if file_size > MAX_FILE_SIZE {
            bail!("File too large ({}MB)", file_size / 1024 / 1024);
        }
        // テキストファイル判定（先頭8KBにNULバイトがなければテキストとみなす）
        let mut buf = [0u8; TEXT_DETECT_BUF_SIZE];
        let n = file
            .read(&mut buf)
            .with_context(|| format!("Failed to read {path}"))?;
        if n > 0 && buf[..n].contains(&0) {
            bail!("Preview is not supported for this file type");
        }
        file.rewind()
            .with_context(|| format!("Failed to seek {path}"))?;
        let reader = BufReader::new(file);
        let mut lines = Vec::new();
        let mut truncated = false;
        for line in reader.lines() {
            let line = line.with_context(|| format!("Failed to read {path}"))?;
            lines.push(line);
            if lines.len() >= MAX_PREVIEW_LINES {
                truncated = true;
                break;
            }
        }
        Ok(Self { lines, truncated })
    }
}
