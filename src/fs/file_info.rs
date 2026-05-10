use crate::fs::VFile;
use anyhow::{Context, Result};
use num_format::{Locale, ToFormattedString};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

pub struct FileInfo {
    pub entries: Vec<(&'static str, String)>,
}

impl FileInfo {
    pub fn from_file(file: &VFile) -> Result<Self> {
        let path = file.absolute_path();
        let metadata = file.metadata()?;
        let mut entries = Vec::new();

        // 共通項目
        entries.push(("Path", path.to_string()));
        entries.push(("Size", metadata.formatted_size()));

        // VFile::metadataはシンボリックリンクを辿るため、symlink_metadataで別途判定
        let is_symlink = std::fs::symlink_metadata(path)
            .map(|m| m.is_symlink())
            .unwrap_or(false);

        if is_symlink {
            entries.push(("Type", "Symlink".to_string()));
            if let Ok(target) = std::fs::read_link(path) {
                entries.push(("Link Target", target.to_string_lossy().to_string()));
            }
        } else if metadata.is_dir() {
            entries.push(("Type", "Directory".to_string()));
            if let Ok(count) = count_dir_entries(path) {
                entries.push(("Items", count.to_formatted_string(&Locale::en)));
            }
        } else {
            let detected = detect_file_kind(path);
            entries.push(("Type", detected.kind.label().to_string()));
            append_kind_specific_entries(&mut entries, path, &detected);
        }

        // 共通: 日時・パーミッション
        entries.push(("Permissions", metadata.permissions().to_rwx_string()));
        if let Ok(created) = metadata.created() {
            entries.push(("Created", created.to_string()));
        }
        if let Ok(modified) = metadata.modified() {
            entries.push(("Modified", modified.to_string()));
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

struct DetectedFile {
    kind: FileKindLabel,
    infer_type: Option<infer::Type>,
}

enum FileKindLabel {
    Text,
    Image,
    Video,
    Audio,
    Binary,
}

impl FileKindLabel {
    fn label(&self) -> &'static str {
        match self {
            FileKindLabel::Text => "Text",
            FileKindLabel::Image => "Image",
            FileKindLabel::Video => "Video",
            FileKindLabel::Audio => "Audio",
            FileKindLabel::Binary => "Binary",
        }
    }
}

fn detect_file_kind(path: &str) -> DetectedFile {
    let Some(infer_type) = infer::get_from_path(path).ok().flatten() else {
        return DetectedFile {
            kind: if is_text_file(path) {
                FileKindLabel::Text
            } else {
                FileKindLabel::Binary
            },
            infer_type: None,
        };
    };

    let mime = infer_type.mime_type();
    let kind = if mime.starts_with("image/") {
        FileKindLabel::Image
    } else if mime.starts_with("video/") {
        FileKindLabel::Video
    } else if mime.starts_with("audio/") {
        FileKindLabel::Audio
    } else if is_text_file(path) {
        FileKindLabel::Text
    } else {
        FileKindLabel::Binary
    };

    DetectedFile {
        kind,
        infer_type: Some(infer_type),
    }
}

const TEXT_DETECT_BUF_SIZE: usize = 8192;

fn is_text_file(path: &str) -> bool {
    let Ok(mut file) = File::open(path) else {
        return false;
    };
    let mut buf = [0u8; TEXT_DETECT_BUF_SIZE];
    let Ok(n) = file.read(&mut buf) else {
        return false;
    };
    if n == 0 {
        return true;
    }
    // NULバイトがなければテキストとみなす
    !buf[..n].contains(&0)
}

fn append_kind_specific_entries(
    entries: &mut Vec<(&'static str, String)>,
    path: &str,
    detected: &DetectedFile,
) {
    match detected.kind {
        FileKindLabel::Text => append_text_entries(entries, path),
        FileKindLabel::Image => append_image_entries(entries, path),
        FileKindLabel::Video | FileKindLabel::Audio => {
            append_media_entries(entries, path, &detected.infer_type)
        }
        FileKindLabel::Binary => append_binary_entries(entries, &detected.infer_type),
    }
}

fn append_text_entries(entries: &mut Vec<(&'static str, String)>, path: &str) {
    if let Ok((line_count, char_count)) = count_text_stats(path) {
        entries.push(("Lines", line_count.to_formatted_string(&Locale::en)));
        entries.push(("Characters", char_count.to_formatted_string(&Locale::en)));
    }
    if let Ok(encoding) = detect_encoding(path) {
        entries.push(("Encoding", encoding));
    }
}

fn append_image_entries(entries: &mut Vec<(&'static str, String)>, path: &str) {
    if let Ok(size) = imagesize::size(path) {
        let ext = Path::new(path)
            .extension()
            .map(|e| e.to_string_lossy().to_uppercase())
            .unwrap_or_default();
        entries.push(("Format", ext));
        entries.push(("Dimensions", format!("{} x {} px", size.width, size.height)));
    }
}

fn append_media_entries(
    entries: &mut Vec<(&'static str, String)>,
    path: &str,
    infer_type: &Option<infer::Type>,
) {
    entries.push(("Format", format_name_from(path, infer_type)));
    if let Some(duration) = get_media_duration(path) {
        entries.push(("Duration", format_duration(duration)));
    }
}

fn append_binary_entries(
    entries: &mut Vec<(&'static str, String)>,
    infer_type: &Option<infer::Type>,
) {
    if let Some(kind) = infer_type {
        entries.push(("MIME Type", kind.mime_type().to_string()));
    }
}

fn count_text_stats(path: &str) -> Result<(usize, usize)> {
    let file = File::open(path).context("Failed to open file")?;
    let reader = BufReader::new(file);
    let mut lines = 0;
    let mut chars = 0;
    for line in reader.lines() {
        let line = line.context("Failed to read line")?;
        chars += line.chars().count();
        lines += 1;
    }
    Ok((lines, chars))
}

const ENCODING_DETECT_BUF_SIZE: usize = 64 * 1024;

fn detect_encoding(path: &str) -> Result<String> {
    let mut file = File::open(path).context("Failed to open file")?;
    let mut buf = vec![0u8; ENCODING_DETECT_BUF_SIZE];
    let n = file.read(&mut buf).context("Failed to read file")?;
    buf.truncate(n);

    let mut detector = chardetng::EncodingDetector::new(chardetng::Iso2022JpDetection::Allow);
    detector.feed(&buf, true);
    let encoding = detector.guess(None, chardetng::Utf8Detection::Allow);
    Ok(encoding.name().to_string())
}

fn format_name_from(path: &str, infer_type: &Option<infer::Type>) -> String {
    infer_type
        .as_ref()
        .map(|t| t.extension().to_uppercase())
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
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
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

fn count_dir_entries(path: &str) -> Result<usize> {
    let count = std::fs::read_dir(path)
        .context("Failed to read directory")?
        .count();
    Ok(count)
}
