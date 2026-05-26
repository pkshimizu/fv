use crate::fs::file_metadata::VFileMetadata;
use anyhow::{Context, Result};
use std::fs::{create_dir, read_dir, rename};
use std::io::{Read, Write};
use std::os::unix::fs::symlink;
use std::path::{Component, Path, PathBuf};

/// ファイルコピー処理の進捗。
/// 全体のファイル数進捗と、現在コピー中のファイルのバイト進捗を表す。
#[derive(Debug, Clone, Copy)]
pub struct CopyProgress {
    pub copied_files: usize,
    pub total_files: usize,
    pub current_bytes: u64,
    pub current_total_bytes: u64,
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
pub fn copy_files_with_progress(
    files: &[VFile],
    dest: &str,
    mut on_progress: impl FnMut(CopyProgress),
) -> Result<()> {
    let total_files = count_files(files)?;
    let mut copied_files = 0usize;
    on_progress(CopyProgress {
        copied_files,
        total_files,
        current_bytes: 0,
        current_total_bytes: 0,
    });
    for file in files {
        let src = Path::new(file.absolute_path());
        let dest_path = resolve_dest_path(src, dest, file.absolute_path())?;
        copy_entry_with_progress(
            src,
            &dest_path,
            &mut copied_files,
            total_files,
            &mut on_progress,
        )?;
    }
    Ok(())
}

fn count_files(files: &[VFile]) -> Result<usize> {
    let mut total = 0;
    for f in files {
        total += count_files_at(Path::new(f.absolute_path()))?;
    }
    Ok(total)
}

fn count_files_at(path: &Path) -> Result<usize> {
    let file_type = std::fs::symlink_metadata(path)
        .with_context(|| format!("{}: Failed to read metadata", path.display()))?
        .file_type();
    if file_type.is_dir() && !file_type.is_symlink() {
        count_files_recursive(path)
    } else {
        Ok(1)
    }
}

fn count_files_recursive(dir: &Path) -> Result<usize> {
    let mut count = 0;
    for entry in
        read_dir(dir).with_context(|| format!("{}: Failed to read directory", dir.display()))?
    {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let entry_path = entry.path();
        if file_type.is_dir() && !file_type.is_symlink() {
            count += count_files_recursive(&entry_path)?;
        } else {
            count += 1;
        }
    }
    Ok(count)
}

fn copy_entry_with_progress(
    src: &Path,
    dest: &Path,
    copied_files: &mut usize,
    total_files: usize,
    on_progress: &mut impl FnMut(CopyProgress),
) -> Result<()> {
    let file_type = std::fs::symlink_metadata(src)
        .with_context(|| format!("{}: Failed to read metadata", src.display()))?
        .file_type();
    if file_type.is_symlink() {
        copy_symlink(src, dest, copied_files, total_files, on_progress)
    } else if file_type.is_dir() {
        copy_dir_with_progress(src, dest, copied_files, total_files, on_progress)
            .with_context(|| format!("{}: Failed to copy directory", dest.display()))
    } else {
        copy_file_with_progress(src, dest, copied_files, total_files, on_progress)
    }
}

fn copy_dir_with_progress(
    src: &Path,
    dest: &Path,
    copied_files: &mut usize,
    total_files: usize,
    on_progress: &mut impl FnMut(CopyProgress),
) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let entry_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if file_type.is_symlink() {
            copy_symlink(
                &entry_path,
                &dest_path,
                copied_files,
                total_files,
                on_progress,
            )?;
        } else if file_type.is_dir() {
            copy_dir_with_progress(
                &entry_path,
                &dest_path,
                copied_files,
                total_files,
                on_progress,
            )?;
        } else {
            copy_file_with_progress(
                &entry_path,
                &dest_path,
                copied_files,
                total_files,
                on_progress,
            )?;
        }
    }
    Ok(())
}

fn copy_symlink(
    src: &Path,
    dest: &Path,
    copied_files: &mut usize,
    total_files: usize,
    on_progress: &mut impl FnMut(CopyProgress),
) -> Result<()> {
    let target = std::fs::read_link(src)
        .with_context(|| format!("{}: Failed to read symlink", src.display()))?;
    symlink(&target, dest)
        .with_context(|| format!("{}: Failed to create symlink", dest.display()))?;
    *copied_files += 1;
    on_progress(CopyProgress {
        copied_files: *copied_files,
        total_files,
        current_bytes: 0,
        current_total_bytes: 0,
    });
    Ok(())
}

fn copy_file_with_progress(
    src: &Path,
    dest: &Path,
    copied_files: &mut usize,
    total_files: usize,
    on_progress: &mut impl FnMut(CopyProgress),
) -> Result<()> {
    let mut src_file = std::fs::File::open(src)
        .with_context(|| format!("{}: Failed to open file", src.display()))?;
    let total_bytes = src_file.metadata().map(|m| m.len()).unwrap_or(0);
    let mut dest_file = std::fs::File::create(dest)
        .with_context(|| format!("{}: Failed to create file", dest.display()))?;
    on_progress(CopyProgress {
        copied_files: *copied_files,
        total_files,
        current_bytes: 0,
        current_total_bytes: total_bytes,
    });
    let mut buf = vec![0u8; 64 * 1024];
    let mut copied = 0u64;
    loop {
        let n = src_file
            .read(&mut buf)
            .with_context(|| format!("{}: Failed to read file", src.display()))?;
        if n == 0 {
            break;
        }
        dest_file
            .write_all(&buf[..n])
            .with_context(|| format!("{}: Failed to write file", dest.display()))?;
        copied += n as u64;
        on_progress(CopyProgress {
            copied_files: *copied_files,
            total_files,
            current_bytes: copied,
            current_total_bytes: total_bytes,
        });
    }
    *copied_files += 1;
    on_progress(CopyProgress {
        copied_files: *copied_files,
        total_files,
        current_bytes: copied,
        current_total_bytes: total_bytes,
    });
    Ok(())
}

/// move_to で EXDEV フォールバック時に使う簡易コピー（進捗通知なし）。
/// シンボリックリンクは辿らず、リンク自体を再作成する。
fn copy_path_simple(src: &Path, dest: &Path) -> Result<()> {
    let file_type = std::fs::symlink_metadata(src)
        .with_context(|| format!("{}: Failed to read metadata", src.display()))?
        .file_type();
    if file_type.is_symlink() {
        let target = std::fs::read_link(src)
            .with_context(|| format!("{}: Failed to read symlink", src.display()))?;
        symlink(&target, dest)
            .with_context(|| format!("{}: Failed to create symlink", dest.display()))?;
    } else if file_type.is_dir() {
        copy_dir_simple(src, dest)
            .with_context(|| format!("{}: Failed to copy directory", dest.display()))?;
    } else {
        std::fs::copy(src, dest)
            .with_context(|| format!("{}: Failed to copy file", dest.display()))?;
    }
    Ok(())
}

fn copy_dir_simple(src: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let entry_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if file_type.is_symlink() {
            let target = std::fs::read_link(&entry_path)
                .with_context(|| format!("{}: Failed to read symlink", entry_path.display()))?;
            symlink(&target, &dest_path)
                .with_context(|| format!("{}: Failed to create symlink", dest_path.display()))?;
        } else if file_type.is_dir() {
            copy_dir_simple(&entry_path, &dest_path)?;
        } else {
            std::fs::copy(&entry_path, &dest_path)
                .with_context(|| format!("{}: Failed to copy file", dest_path.display()))?;
        }
    }
    Ok(())
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
