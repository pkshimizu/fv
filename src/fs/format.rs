/// バイト数を人が読みやすい形式 (例: `1.5 MB`, `512 B`) に整形する。
///
/// `u64 -> f64` のキャストは 2^53 (~8 PB) 以下の値で表示精度を保つ。
/// 実用上の単一ファイルサイズは GB オーダで十分なので問題にならない。
#[allow(clippy::cast_precision_loss)]
pub(crate) fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}
