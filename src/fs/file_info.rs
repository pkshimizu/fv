use crate::fs::VFile;
use anyhow::{Context, Result};
use num_format::{Locale, ToFormattedString};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

pub struct FileInfo {
    pub entries: Vec<(String, String)>,
}

impl FileInfo {
    pub fn from_file(file: &VFile) -> Result<Self> {
        let path = file.absolute_path();
        let metadata = file.metadata()?;
        let mut entries = Vec::new();

        // 共通項目
        entries.push(("Path".to_string(), path.to_string()));
        entries.push((
            "Size".to_string(),
            format_size(metadata.file_size()),
        ));

        if metadata.is_symlink() {
            entries.push(("Type".to_string(), "Symlink".to_string()));
            if let Ok(target) = std::fs::read_link(path) {
                entries.push(("Link Target".to_string(), target.to_string_lossy().to_string()));
            }
        } else if metadata.is_dir() {
            entries.push(("Type".to_string(), "Directory".to_string()));
            if let Ok(count) = count_dir_entries(path) {
                entries.push(("Items".to_string(), count.to_formatted_string(&Locale::en)));
            }
        } else {
            let file_kind = detect_file_kind(path);
            entries.push(("Type".to_string(), file_kind.label().to_string()));
            append_kind_specific_entries(&mut entries, path, &file_kind);
        }

        // 共通: 日時・パーミッション
        entries.push((
            "Permissions".to_string(),
            metadata.permissions().to_rwx_string(),
        ));
        if let Ok(created) = metadata.created() {
            entries.push(("Created".to_string(), created.to_string()));
        }
        if let Ok(modified) = metadata.modified() {
            entries.push(("Modified".to_string(), modified.to_string()));
        }

        Ok(Self { entries })
    }

    pub fn to_lines(&self) -> Vec<String> {
        let label_width = self.entries.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
        self.entries
            .iter()
            .map(|(k, v)| format!("{k:label_width$}  {v}"))
            .collect()
    }
}

enum FileKind {
    Text,
    Image,
    Video,
    Audio,
    Binary,
}

impl FileKind {
    fn label(&self) -> &'static str {
        match self {
            FileKind::Text => "Text",
            FileKind::Image => "Image",
            FileKind::Video => "Video",
            FileKind::Audio => "Audio",
            FileKind::Binary => "Binary",
        }
    }
}

fn detect_file_kind(path: &str) -> FileKind {
    let Some(kind) = infer::get_from_path(path).ok().flatten() else {
        return if is_text_file(path) {
            FileKind::Text
        } else {
            FileKind::Binary
        };
    };

    let mime = kind.mime_type();
    if mime.starts_with("image/") {
        FileKind::Image
    } else if mime.starts_with("video/") {
        FileKind::Video
    } else if mime.starts_with("audio/") {
        FileKind::Audio
    } else if is_text_file(path) {
        FileKind::Text
    } else {
        FileKind::Binary
    }
}

fn is_text_file(path: &str) -> bool {
    let Ok(mut file) = File::open(path) else {
        return false;
    };
    let mut buf = [0u8; 8192];
    let Ok(n) = file.read(&mut buf) else {
        return false;
    };
    if n == 0 {
        return true;
    }
    // NULバイトがなければテキストとみなす
    !buf[..n].contains(&0)
}

fn append_kind_specific_entries(entries: &mut Vec<(String, String)>, path: &str, kind: &FileKind) {
    match kind {
        FileKind::Text => append_text_entries(entries, path),
        FileKind::Image => append_image_entries(entries, path),
        FileKind::Video | FileKind::Audio => append_media_entries(entries, path),
        FileKind::Binary => append_binary_entries(entries, path),
    }
}

fn append_text_entries(entries: &mut Vec<(String, String)>, path: &str) {
    if let Ok((line_count, char_count)) = count_text_stats(path) {
        entries.push(("Lines".to_string(), line_count.to_formatted_string(&Locale::en)));
        entries.push(("Characters".to_string(), char_count.to_formatted_string(&Locale::en)));
    }
    if let Ok(encoding) = detect_encoding(path) {
        entries.push(("Encoding".to_string(), encoding));
    }
}

fn append_image_entries(entries: &mut Vec<(String, String)>, path: &str) {
    if let Ok(size) = imagesize::size(path) {
        entries.push(("Format".to_string(), detect_format_name(path)));
        entries.push(("Dimensions".to_string(), format!("{} x {} px", size.width, size.height)));
    }
}

fn append_media_entries(entries: &mut Vec<(String, String)>, path: &str) {
    entries.push(("Format".to_string(), detect_format_name(path)));
    if let Some(duration) = get_media_duration(path) {
        entries.push(("Duration".to_string(), format_duration(duration)));
    }
}

fn append_binary_entries(entries: &mut Vec<(String, String)>, path: &str) {
    if let Some(kind) = infer::get_from_path(path).ok().flatten() {
        entries.push(("MIME Type".to_string(), kind.mime_type().to_string()));
    }
}

fn count_text_stats(path: &str) -> Result<(usize, usize)> {
    let file = File::open(path).context("Failed to open file")?;
    let reader = BufReader::new(file);
    let mut lines = 0;
    let mut chars = 0;
    for line in reader.lines() {
        let line = line.context("Failed to read line")?;
        chars += line.len();
        lines += 1;
    }
    Ok((lines, chars))
}

fn detect_encoding(path: &str) -> Result<String> {
    let mut file = File::open(path).context("Failed to open file")?;
    let mut buf = vec![0u8; 64 * 1024];
    let n = file.read(&mut buf).context("Failed to read file")?;
    buf.truncate(n);

    let mut detector =
        chardetng::EncodingDetector::new(chardetng::Iso2022JpDetection::Allow);
    detector.feed(&buf, true);
    let encoding = detector.guess(None, chardetng::Utf8Detection::Allow);
    Ok(encoding.name().to_string())
}

fn detect_format_name(path: &str) -> String {
    infer::get_from_path(path)
        .ok()
        .flatten()
        .map(|kind| kind.extension().to_uppercase())
        .unwrap_or_else(|| {
            Path::new(path)
                .extension()
                .map(|e| e.to_string_lossy().to_uppercase())
                .unwrap_or_default()
        })
}

fn get_media_duration(path: &str) -> Option<f64> {
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let file = File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = Path::new(path).extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .ok()?;

    let reader = probed.format;
    let track = reader.default_track()?;
    let time_base = track.codec_params.time_base?;
    let n_frames = track.codec_params.n_frames?;
    let duration = time_base.calc_time(n_frames);

    Some(duration.seconds as f64 + duration.frac)
}

fn format_duration(seconds: f64) -> String {
    let total = seconds as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    let formatted = bytes.to_formatted_string(&Locale::en);
    if bytes >= GB {
        format!("{formatted} bytes ({:.1} GB)", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{formatted} bytes ({:.1} MB)", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{formatted} bytes ({:.1} KB)", bytes as f64 / KB as f64)
    } else {
        format!("{formatted} bytes")
    }
}

fn count_dir_entries(path: &str) -> Result<usize> {
    let count = std::fs::read_dir(path)
        .context("Failed to read directory")?
        .count();
    Ok(count)
}
