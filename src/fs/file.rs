use crate::fs::file_metadata::VFileMetadata;
use anyhow::{Context, Result};
use std::fs::{create_dir, read_dir, rename};
use std::path::{Component, Path, PathBuf};

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

    pub fn delete(&self) -> Result<()> {
        let path = self.absolute_path();
        trash::delete(path).with_context(|| format!("{}: Failed to trash", self.path))?;

        Ok(())
    }

    pub fn copy_to(&self, path: &str) -> Result<()> {
        let dest = Path::new(path);
        let src = Path::new(self.absolute_path());

        let dest_path = if dest.is_dir() {
            let file_name = src
                .file_name()
                .with_context(|| format!("{}: No file name", self.path))?;
            unique_path(&dest.join(file_name))?
        } else if dest.exists() {
            unique_path(dest)?
        } else {
            dest.to_path_buf()
        };

        if src.is_dir() {
            copy_dir_recursive(src, &dest_path)
                .with_context(|| format!("{}: Failed to copy directory", dest_path.display()))?;
        } else {
            std::fs::copy(src, &dest_path)
                .with_context(|| format!("{}: Failed to copy file", dest_path.display()))?;
        }

        Ok(())
    }

    pub fn move_to(&self, path: &str) -> Result<()> {
        let dest = Path::new(path);
        let src = Path::new(self.absolute_path());

        let dest_path = if dest.is_dir() {
            let file_name = src
                .file_name()
                .with_context(|| format!("{}: No file name", self.path))?;
            unique_path(&dest.join(file_name))?
        } else if dest.exists() {
            unique_path(dest)?
        } else {
            dest.to_path_buf()
        };

        rename(src, &dest_path)
            .with_context(|| format!("{}: Failed to move file", dest_path.display()))?;

        Ok(())
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

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let entry_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&entry_path, &dest_path)?;
        } else {
            // シンボリックリンクはリンク先の内容をファイルとしてコピーする
            std::fs::copy(&entry_path, &dest_path)
                .with_context(|| format!("{}: Failed to copy file", dest_path.display()))?;
        }
    }
    Ok(())
}
