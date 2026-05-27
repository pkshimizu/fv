use crate::fs::file_metadata::VFileMetadata;
use anyhow::{Context, Result};
use std::fs::{FileType, create_dir, read_dir, rename};
use std::io::{Read, Write};
use std::os::unix::fs::{OpenOptionsExt, symlink};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const COPY_BUFFER_SIZE: usize = 256 * 1024;

/// 1 ファイルのコピー中に進捗を通知する最小バイト間隔。
/// 64KB チャンク毎ではなく一定の累積バイト数ごとに通知することで、
/// mpsc 送信やフォーマット処理のオーバーヘッドを抑える。
const PROGRESS_NOTIFY_BYTES: u64 = 1024 * 1024;

/// `AtomicUsize` のセンチネル値。総ファイル数がまだ確定していないことを表す。
const TOTAL_FILES_UNKNOWN: usize = usize::MAX;

/// 再帰的なディレクトリトラバーサルで許容する最大階層深さ。
/// ハードリンクされたディレクトリ循環や、想定外に深いツリーで
/// スタックオーバーフローを起こす前に `anyhow::bail!` で停止するための安全網。
/// 通常のファイルシステムでは PATH_MAX (~4096) に近い深さでも安全。
const MAX_DIR_DEPTH: usize = 4096;

/// ファイルコピー処理の進捗。
/// 全体のファイル数進捗と、現在コピー中のファイルのバイト進捗を表す。
/// `total_files` はバックグラウンドで集計されるため、確定するまでは `None`。
#[derive(Debug, Clone, Copy)]
pub(crate) struct CopyProgress {
    pub copied_files: usize,
    pub total_files: Option<usize>,
    pub current_bytes: u64,
    pub current_total_bytes: u64,
}

fn load_total_files(total: &AtomicUsize) -> Option<usize> {
    let v = total.load(Ordering::Acquire);
    (v != TOTAL_FILES_UNKNOWN).then_some(v)
}

/// コピー処理中に持ち回る状態とコールバックをまとめた構造体。
/// 引数の数を抑え、進捗通知やキャンセル判定のロジックを集約する。
/// `buf` はストリーミングコピー用のバッファ。1 回のコピー操作全体で再利用される。
struct CopyContext<'a> {
    copied_files: usize,
    total_files: &'a AtomicUsize,
    cancel: &'a AtomicBool,
    buf: &'a mut [u8],
    on_progress: &'a mut dyn FnMut(CopyProgress),
}

impl CopyContext<'_> {
    fn notify(&mut self, current_bytes: u64, current_total_bytes: u64) {
        (self.on_progress)(CopyProgress {
            copied_files: self.copied_files,
            total_files: load_total_files(self.total_files),
            current_bytes,
            current_total_bytes,
        });
    }

    /// 1 ファイルのコピー完了時に呼ぶ。コピー済み数を増やしてから進捗を通知する。
    fn complete_one(&mut self, current_bytes: u64, current_total_bytes: u64) {
        self.copied_files += 1;
        self.notify(current_bytes, current_total_bytes);
    }

    fn check_cancel(&self) -> Result<()> {
        if self.cancel.load(Ordering::Relaxed) {
            anyhow::bail!("Copy canceled");
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct VFile {
    path: String,
    metadata: Option<VFileMetadata>,
}

// VFileMetadataのMetadataがPartialEqを実装していないため、pathのみでeqを実装
impl PartialEq for VFile {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Eq for VFile {}

impl VFile {
    pub fn new(path: impl Into<String>) -> Self {
        let path = path.into();
        let metadata = std::fs::metadata(&path).ok().map(VFileMetadata::new);
        Self { path, metadata }
    }

    pub fn absolute_path(&self) -> &str {
        &self.path
    }

    pub fn file_name(&self) -> Option<&str> {
        Path::new(&self.path).file_name()?.to_str()
    }

    pub fn parent_dir(&self) -> Option<VFile> {
        let path = Path::new(&self.path);
        let parent = path.parent()?;
        let parent_path = parent.to_str()?;
        Some(VFile::new(parent_path))
    }

    pub fn list(&self) -> Result<Vec<VFile>> {
        let result = read_dir(&self.path)?;
        let mut files: Vec<VFile> = Vec::new();
        for entry in result {
            let path = entry?.path();
            if let Some(path_str) = path.to_str() {
                files.push(VFile::new(path_str));
            }
        }
        Ok(files)
    }

    pub fn metadata(&self) -> Result<&VFileMetadata> {
        self.metadata
            .as_ref()
            .with_context(|| format!("{}: No metadata", self.path))
    }

    pub fn is_dir(&self) -> bool {
        self.metadata.as_ref().is_some_and(|m| m.is_dir())
    }

    pub fn create_file(&self, file_name: &str) -> Result<()> {
        if file_name.is_empty() {
            return Ok(());
        }
        anyhow::ensure!(
            Path::new(file_name)
                .components()
                .all(|c| matches!(c, Component::Normal(_))),
            "{file_name}: Invalid file name"
        );
        let path = Path::new(self.absolute_path());
        let file_path = path.join(file_name);
        anyhow::ensure!(
            !file_path.exists(),
            "{}: File already exists",
            file_path.display()
        );
        std::fs::File::create(&file_path)
            .with_context(|| format!("{}: Failed to create file", file_path.display()))?;
        Ok(())
    }

    pub fn create_dir(&self, dir_name: &str) -> Result<()> {
        if dir_name.is_empty() {
            return Ok(());
        }
        anyhow::ensure!(
            Path::new(dir_name)
                .components()
                .all(|c| matches!(c, Component::Normal(_))),
            "{dir_name}: Invalid dir name"
        );
        let path = Path::new(self.absolute_path());
        let dir_path = path.join(dir_name);
        create_dir(&dir_path)
            .with_context(|| format!("{}: Failed to create directory", dir_path.display()))?;

        Ok(())
    }

    pub fn rename(&self, name: &str) -> Result<()> {
        if name.is_empty() {
            return Ok(());
        }
        anyhow::ensure!(
            Path::new(name)
                .components()
                .all(|c| matches!(c, Component::Normal(_))),
            "{name}: Invalid file name"
        );
        let path = Path::new(self.absolute_path());
        let new_path = path
            .parent()
            .context("Failed to get parent path")?
            .join(name);
        anyhow::ensure!(
            !new_path.exists(),
            "{}: File already exists",
            new_path.display()
        );
        rename(path, &new_path)
            .with_context(|| format!("{}: Failed to rename file", new_path.display()))?;

        Ok(())
    }

    pub fn create_zip(&self, zip_name: &str, files: &[VFile]) -> Result<()> {
        if zip_name.is_empty() {
            return Ok(());
        }
        anyhow::ensure!(
            Path::new(zip_name)
                .components()
                .all(|c| matches!(c, Component::Normal(_))),
            "{zip_name}: Invalid file name"
        );
        let zip_path = Path::new(self.absolute_path()).join(zip_name);
        let unique_zip_path = unique_path(&zip_path)?;
        let zip_file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&unique_zip_path)
            .with_context(|| format!("{}: Failed to create zip file", unique_zip_path.display()))?;

        let result = write_zip(zip_file, files);
        if result.is_err() {
            let _ = std::fs::remove_file(&unique_zip_path);
        }
        result
    }

    pub fn extract_zip(&self, dest_dir: &str) -> Result<()> {
        let zip_file = std::fs::File::open(self.absolute_path())
            .with_context(|| format!("{}: Failed to open zip file", self.path))?;
        let mut archive = zip::ZipArchive::new(zip_file)
            .with_context(|| format!("{}: Failed to read zip archive", self.path))?;
        let dest = Path::new(dest_dir);
        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .with_context(|| format!("{}: Failed to read zip entry", self.path))?;
            let Some(enclosed_name) = entry.enclosed_name() else {
                continue;
            };
            let out_path = dest.join(enclosed_name);
            if entry.is_dir() {
                std::fs::create_dir_all(&out_path).with_context(|| {
                    format!("{}: Failed to create directory", out_path.display())
                })?;
            } else if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("{}: Failed to create directory", parent.display()))?;
                let mut out_file = std::fs::File::create(&out_path)
                    .with_context(|| format!("{}: Failed to create file", out_path.display()))?;
                std::io::copy(&mut entry, &mut out_file)
                    .with_context(|| format!("{}: Failed to extract file", out_path.display()))?;
            }
        }
        Ok(())
    }

    pub fn delete(&self) -> Result<()> {
        let path = self.absolute_path();
        trash::delete(path).with_context(|| format!("{}: Failed to trash", self.path))?;

        Ok(())
    }

    pub fn remove(&self) -> Result<()> {
        let path = Path::new(self.absolute_path());
        if path.is_dir() {
            std::fs::remove_dir_all(path)
                .with_context(|| format!("{}: Failed to remove directory", self.path))?;
        } else {
            std::fs::remove_file(path)
                .with_context(|| format!("{}: Failed to remove file", self.path))?;
        }
        Ok(())
    }

    pub fn move_to(&self, path: &str) -> Result<()> {
        let src = Path::new(self.absolute_path());
        let dest_path = resolve_dest_path(src, path, &self.path)?;

        match rename(src, &dest_path) {
            Ok(()) => Ok(()),
            Err(e) if e.raw_os_error() == Some(libc::EXDEV) => {
                copy_path_simple(src, &dest_path)?;
                self.remove()
            }
            Err(e) => Err(anyhow::Error::from(e)
                .context(format!("{}: Failed to move file", dest_path.display()))),
        }
    }
}

/// ファイル群を進捗を通知しながらコピーする。
/// `on_progress` は処理開始時、各ファイルの読み込みチャンク毎、各ファイルのコピー完了時に呼ばれる。
/// シンボリックリンクは辿らず、リンク自体を再作成する。
/// `total_files` は呼び出し側が用意し、別スレッドで集計してこの atomic に書き込む想定（確定まで `TOTAL_FILES_UNKNOWN`）。
/// `cancel` を `true` に設定するとコピー処理が中断され `Err` が返る。
pub(crate) fn copy_files_with_progress(
    files: &[VFile],
    dest: &str,
    cancel: &AtomicBool,
    total_files: &AtomicUsize,
    mut on_progress: impl FnMut(CopyProgress),
) -> Result<()> {
    // 複数ファイルを同一の非ディレクトリパスへコピーすると、2 つ目以降が
    // unique_path で別名にされたり上書きされたりして意図しない結果になる。
    // ディレクトリ必須の前提を明示的に検証する。
    if files.len() > 1 {
        anyhow::ensure!(
            Path::new(dest).is_dir(),
            "{dest}: must be an existing directory when copying multiple files"
        );
    }
    let mut buf = vec![0u8; COPY_BUFFER_SIZE];
    let mut ctx = CopyContext {
        copied_files: 0,
        total_files,
        cancel,
        buf: &mut buf,
        on_progress: &mut on_progress,
    };
    ctx.notify(0, 0);
    for file in files {
        ctx.check_cancel()?;
        let src = Path::new(file.absolute_path());
        let resolved_dest = resolve_dest_path(src, dest, file.absolute_path())?;
        ensure_dest_not_inside_src(src, &resolved_dest)?;
        copy_entry_with_progress(src, &resolved_dest, None, 0, &mut ctx)?;
    }
    Ok(())
}

/// `dest_path` が `src` 自身または `src` 配下に該当する場合エラーを返す。
/// 該当時にコピーを続けると無限再帰でディスクを使い切るため、事前に検出する。
fn ensure_dest_not_inside_src(src: &Path, dest_path: &Path) -> Result<()> {
    // ディレクトリでなければ循環の可能性はない（symlink もリンク自体を再作成するため対象外）
    let is_dir = std::fs::symlink_metadata(src)
        .with_context(|| format!("{}: Failed to read metadata", src.display()))?
        .file_type()
        .is_dir();
    if !is_dir {
        return Ok(());
    }
    let src_canon = std::fs::canonicalize(src)
        .with_context(|| format!("{}: Failed to canonicalize source", src.display()))?;
    // dest_path はまだ存在しない可能性があるので親ディレクトリで canonicalize して結合する
    let dest_parent = dest_path
        .parent()
        .with_context(|| format!("{}: Failed to get parent directory", dest_path.display()))?;
    let dest_parent_canon = std::fs::canonicalize(dest_parent).with_context(|| {
        format!(
            "{}: Failed to canonicalize destination parent",
            dest_parent.display()
        )
    })?;
    let dest_name = dest_path
        .file_name()
        .with_context(|| format!("{}: Failed to get file name", dest_path.display()))?;
    let dest_canon = dest_parent_canon.join(dest_name);
    if dest_canon.starts_with(&src_canon) {
        anyhow::bail!(
            "{}: Cannot copy directory into itself or its subdirectory ({})",
            src.display(),
            dest_path.display()
        );
    }
    Ok(())
}

pub(super) fn count_files(files: &[VFile]) -> Result<usize> {
    let mut total = 0;
    for f in files {
        let path = Path::new(f.absolute_path());
        let file_type = std::fs::symlink_metadata(path)
            .with_context(|| format!("{}: Failed to read metadata", path.display()))?
            .file_type();
        total += count_entries(path, file_type, 0)?;
    }
    Ok(total)
}

/// `path` 配下のファイル/symlink 数を再帰的に数える。
/// `file_type` は `path` 自身の型で、内側ループでは `DirEntry::file_type()` から得て渡すことで
/// `symlink_metadata` の追加 syscall を避ける。
/// FileType の is_dir / is_file / is_symlink は相互排他なので is_dir() のみで判定。
/// `depth` が `MAX_DIR_DEPTH` を超えると bail し、循環や異常に深いツリーで暴走するのを防ぐ。
fn count_entries(path: &Path, file_type: FileType, depth: usize) -> Result<usize> {
    if depth > MAX_DIR_DEPTH {
        anyhow::bail!(
            "{}: directory nesting exceeds {} levels",
            path.display(),
            MAX_DIR_DEPTH
        );
    }
    if !file_type.is_dir() {
        return Ok(1);
    }
    let mut count = 0;
    for entry in
        read_dir(path).with_context(|| format!("{}: Failed to read directory", path.display()))?
    {
        let entry =
            entry.with_context(|| format!("{}: Failed to read directory entry", path.display()))?;
        let entry_path = entry.path();
        let entry_type = entry
            .file_type()
            .with_context(|| format!("{}: Failed to get file type", entry_path.display()))?;
        count += count_entries(&entry_path, entry_type, depth + 1)?;
    }
    Ok(count)
}

/// `file_type` を `Some` で受け取った場合は再フェッチせずそれを使う。
/// トップレベルの呼び出しは `None` を渡し `symlink_metadata` で取得、
/// `copy_dir_with_progress` からの再帰時は `DirEntry::file_type` で得た値を渡すことで
/// 内側エントリでの `symlink_metadata` 重複呼び出しを避ける。
/// `depth` は再帰深さで、`copy_dir_with_progress` で `MAX_DIR_DEPTH` を超えると bail する。
fn copy_entry_with_progress(
    src: &Path,
    dest: &Path,
    file_type: Option<FileType>,
    depth: usize,
    ctx: &mut CopyContext,
) -> Result<()> {
    let file_type = match file_type {
        Some(ft) => ft,
        None => std::fs::symlink_metadata(src)
            .with_context(|| format!("{}: Failed to read metadata", src.display()))?
            .file_type(),
    };
    if file_type.is_symlink() {
        copy_symlink(src, dest, ctx)
    } else if file_type.is_dir() {
        copy_dir_with_progress(src, dest, depth, ctx)
    } else {
        copy_file_with_progress(src, dest, ctx)
    }
}

fn copy_dir_with_progress(
    src: &Path,
    dest: &Path,
    depth: usize,
    ctx: &mut CopyContext,
) -> Result<()> {
    if depth > MAX_DIR_DEPTH {
        anyhow::bail!(
            "{}: directory nesting exceeds {} levels",
            src.display(),
            MAX_DIR_DEPTH
        );
    }
    std::fs::create_dir_all(dest)
        .with_context(|| format!("{}: Failed to create directory", dest.display()))?;
    for entry in
        read_dir(src).with_context(|| format!("{}: Failed to read directory", src.display()))?
    {
        ctx.check_cancel()?;
        let entry =
            entry.with_context(|| format!("{}: Failed to read directory entry", src.display()))?;
        let entry_path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("{}: Failed to get file type", entry_path.display()))?;
        let dest_path = dest.join(entry.file_name());
        copy_entry_with_progress(&entry_path, &dest_path, Some(file_type), depth + 1, ctx)?;
    }
    Ok(())
}

fn copy_symlink(src: &Path, dest: &Path, ctx: &mut CopyContext) -> Result<()> {
    let target = std::fs::read_link(src)
        .with_context(|| format!("{}: Failed to read symlink", src.display()))?;
    symlink(&target, dest)
        .with_context(|| format!("{}: Failed to create symlink", dest.display()))?;
    ctx.complete_one(0, 0);
    Ok(())
}

fn copy_file_with_progress(src: &Path, dest: &Path, ctx: &mut CopyContext) -> Result<()> {
    let total_bytes = std::fs::metadata(src)
        .with_context(|| format!("{}: Failed to read file metadata", src.display()))?
        .len();

    ctx.notify(0, total_bytes);

    // ファイル内バイト進捗の通知間隔より小さいファイルは中間進捗を出さないので、
    // 軽量な io::copy 経路を使う（Linux なら内部で copy_file_range の fast path に乗る）。
    if total_bytes < PROGRESS_NOTIFY_BYTES {
        ctx.check_cancel()?;
        copy_file_fast(src, dest)?;
    } else {
        copy_file_streaming(src, dest, total_bytes, ctx)?;
    }

    ctx.complete_one(total_bytes, total_bytes);
    Ok(())
}

fn copy_file_fast(src: &Path, dest: &Path) -> Result<()> {
    // dest が事前に存在しなかった場合のみ、失敗時に書きかけの dest を削除する。
    // 既存ファイルへの上書きで失敗した場合は、削除すると元データを失うため敢えて残す。
    let mut guard = PartialDestGuard::new(dest);
    let mut src_file = std::fs::File::open(src)
        .with_context(|| format!("{}: Failed to open file", src.display()))?;
    // streaming 経路と一貫させて O_NOFOLLOW を付け、TOCTOU 攻撃を防ぐ。
    // std::fs::copy より少し遅いが、Linux では io::copy が copy_file_range に
    // 特殊化されるため fast path 自体は維持される。
    let mut dest_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(dest)
        .with_context(|| format!("{}: Failed to create file", dest.display()))?;
    std::io::copy(&mut src_file, &mut dest_file)
        .with_context(|| format!("{}: Failed to copy file", dest.display()))?;
    guard.disarm();
    Ok(())
}

fn copy_file_streaming(
    src: &Path,
    dest: &Path,
    total_bytes: u64,
    ctx: &mut CopyContext,
) -> Result<()> {
    let mut src_file = std::fs::File::open(src)
        .with_context(|| format!("{}: Failed to open file", src.display()))?;
    // O_NOFOLLOW を付け、resolve_dest_path との間で dest に symlink を差し込まれた場合に
    // 任意のファイル（/etc/passwd など）を truncate する TOCTOU 攻撃を防ぐ。
    // 通常コピーは regular file または非存在パスにしか書き込まないため副作用なし。
    let mut guard = PartialDestGuard::new(dest);
    let mut dest_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(dest)
        .with_context(|| format!("{}: Failed to create file", dest.display()))?;

    let mut copied = 0u64;
    let mut last_notified = 0u64;
    loop {
        ctx.check_cancel()?;
        let n = src_file
            .read(ctx.buf)
            .with_context(|| format!("{}: Failed to read file", src.display()))?;
        if n == 0 {
            break;
        }
        dest_file
            .write_all(&ctx.buf[..n])
            .with_context(|| format!("{}: Failed to write file", dest.display()))?;
        copied += n as u64;
        if copied.saturating_sub(last_notified) >= PROGRESS_NOTIFY_BYTES {
            ctx.notify(copied, total_bytes);
            last_notified = copied;
        }
    }
    // 生 `File` への `flush()` は no-op だが、将来 `BufWriter` を挟む変更が
    // 入ってもエラー検知できるように残している。
    dest_file
        .flush()
        .with_context(|| format!("{}: Failed to flush file", dest.display()))?;
    guard.disarm();
    Ok(())
}

/// コピー失敗時に書きかけの dest を削除するための RAII ガード。
/// dest が事前に存在していた場合は arm せず、既存ファイルを誤って消さないようにする。
struct PartialDestGuard<'a> {
    path: &'a Path,
    armed: bool,
}

impl<'a> PartialDestGuard<'a> {
    fn new(path: &'a Path) -> Self {
        // symlink_metadata は symlink を辿らないため、dangling symlink も「存在する」と判定できる。
        // 権限エラー等の予期せぬ失敗時は保守的に「存在する」とみなして cleanup を抑止する。
        let pre_existed = match std::fs::symlink_metadata(path) {
            Ok(_) => true,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
            Err(_) => true,
        };
        Self {
            path,
            armed: !pre_existed,
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for PartialDestGuard<'_> {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::fs::remove_file(self.path);
        }
    }
}

/// move_to で EXDEV フォールバック時に使う簡易コピー（進捗通知なし、キャンセル不要）。
/// 進捗通知を no-op にして `copy_entry_with_progress` に委譲することで、
/// シンボリックリンク処理や再帰ロジックを共通化している。
fn copy_path_simple(src: &Path, dest: &Path) -> Result<()> {
    let total_files = AtomicUsize::new(TOTAL_FILES_UNKNOWN);
    let cancel = AtomicBool::new(false);
    let mut buf = vec![0u8; COPY_BUFFER_SIZE];
    let mut ctx = CopyContext {
        copied_files: 0,
        total_files: &total_files,
        cancel: &cancel,
        buf: &mut buf,
        on_progress: &mut |_| {},
    };
    copy_entry_with_progress(src, dest, None, 0, &mut ctx)
}

fn resolve_dest_path(src: &Path, path: &str, src_display: &str) -> Result<PathBuf> {
    let dest = Path::new(path);
    if dest.is_dir() {
        let file_name = src
            .file_name()
            .with_context(|| format!("{src_display}: No file name"))?;
        unique_path(&dest.join(file_name))
    } else if dest.exists() {
        unique_path(dest)
    } else {
        Ok(dest.to_path_buf())
    }
}

const MAX_UNIQUE_PATH_SUFFIX: u32 = 1000;

fn unique_path(path: &Path) -> Result<PathBuf> {
    if !path.exists() {
        return Ok(path.to_path_buf());
    }

    let parent = path.parent().context("Failed to get parent directory")?;
    let stem = path
        .file_stem()
        .context("Failed to get file stem")?
        .to_string_lossy();
    let ext = path.extension().map(|e| e.to_string_lossy());

    for i in 1..=MAX_UNIQUE_PATH_SUFFIX {
        let new_name = match &ext {
            Some(ext) => format!("{stem}_{i}.{ext}"),
            None => format!("{stem}_{i}"),
        };
        let candidate = parent.join(&new_name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    anyhow::bail!("Failed to make unique path")
}

fn write_zip(zip_file: std::fs::File, files: &[VFile]) -> Result<()> {
    let mut zip_writer = zip::ZipWriter::new(zip_file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for file in files {
        let file_path = Path::new(file.absolute_path());
        if file.is_dir() {
            let prefix = file_path.parent().unwrap_or(file_path);
            add_dir_to_zip(&mut zip_writer, prefix, file_path, options)?;
        } else {
            let name = file.file_name().context("Failed to get file name")?;
            add_file_to_zip(&mut zip_writer, file_path, name, options)?;
        }
    }
    zip_writer.finish().context("Failed to finalize zip file")?;
    Ok(())
}

fn add_dir_to_zip(
    zip_writer: &mut zip::ZipWriter<std::fs::File>,
    prefix: &Path,
    dir: &Path,
    options: zip::write::SimpleFileOptions,
) -> Result<()> {
    for entry in
        read_dir(dir).with_context(|| format!("{}: Failed to read directory", dir.display()))?
    {
        let entry =
            entry.with_context(|| format!("{}: Failed to read directory entry", dir.display()))?;
        let file_type = entry
            .file_type()
            .with_context(|| format!("{}: Failed to get file type", entry.path().display()))?;
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        let relative = path.strip_prefix(prefix).unwrap_or(&path);
        let name = relative.to_string_lossy();
        if file_type.is_dir() {
            zip_writer
                .add_directory(format!("{name}/"), options)
                .with_context(|| format!("Failed to add directory {name} to zip"))?;
            add_dir_to_zip(zip_writer, prefix, &path, options)?;
        } else {
            add_file_to_zip(zip_writer, &path, &name, options)?;
        }
    }
    Ok(())
}

fn add_file_to_zip(
    zip_writer: &mut zip::ZipWriter<std::fs::File>,
    file_path: &Path,
    zip_name: &str,
    options: zip::write::SimpleFileOptions,
) -> Result<()> {
    zip_writer
        .start_file(zip_name, options)
        .with_context(|| format!("Failed to add {zip_name} to zip"))?;
    let mut f = std::fs::File::open(file_path)
        .with_context(|| format!("{}: Failed to open file", file_path.display()))?;
    std::io::copy(&mut f, zip_writer)
        .with_context(|| format!("{}: Failed to write to zip", file_path.display()))?;
    Ok(())
}
