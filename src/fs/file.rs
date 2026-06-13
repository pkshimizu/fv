use crate::fs::file_metadata::VFileMetadata;
use anyhow::{Context, Result};
use std::fs::{create_dir, read_dir, rename};
use std::path::{Component, Path};

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

    pub fn file_stem(&self) -> Option<&str> {
        Path::new(&self.path).file_stem()?.to_str()
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

    /// `self`（ディレクトリ）の中に、`target`（絶対パス）を指すシンボリックリンクを
    /// `link_name` で作成する。既存パスがあれば作成しない（衝突回避は他の作成系と同方針）。
    pub fn create_symlink(&self, link_name: &str, target: &str) -> Result<()> {
        if link_name.is_empty() {
            return Ok(());
        }
        anyhow::ensure!(
            Path::new(link_name)
                .components()
                .all(|c| matches!(c, Component::Normal(_))),
            "{link_name}: Invalid link name"
        );
        let link_path = Path::new(self.absolute_path()).join(link_name);
        anyhow::ensure!(
            !link_path.exists(),
            "{}: File already exists",
            link_path.display()
        );
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, &link_path)
                .with_context(|| format!("{}: Failed to create symlink", link_path.display()))?;
        }
        #[cfg(not(unix))]
        {
            let _ = target;
            anyhow::bail!("シンボリックリンクの作成はこのプラットフォームでは未対応です");
        }
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
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn create_symlink_creates_link_pointing_to_target() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("real.txt");
        std::fs::write(&target, "hello").unwrap();

        let dir_vfile = VFile::new(dir.path().to_str().unwrap());
        dir_vfile
            .create_symlink("link.txt", target.to_str().unwrap())
            .unwrap();

        let link = dir.path().join("link.txt");
        assert!(
            link.symlink_metadata().unwrap().file_type().is_symlink(),
            "シンボリックリンクとして作成される"
        );
        // リンク経由でターゲットの内容が読める。
        assert_eq!(std::fs::read_to_string(&link).unwrap(), "hello");
    }

    #[test]
    fn create_symlink_errors_when_link_name_already_exists() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("exists.txt"), "").unwrap();

        let dir_vfile = VFile::new(dir.path().to_str().unwrap());
        assert!(
            dir_vfile
                .create_symlink("exists.txt", "/tmp/whatever")
                .is_err()
        );
    }

    #[test]
    fn create_symlink_with_empty_name_is_noop() {
        let dir = TempDir::new().unwrap();
        let dir_vfile = VFile::new(dir.path().to_str().unwrap());
        // 空名は何もしない（既存の create 系と同じ）。
        assert!(dir_vfile.create_symlink("", "/tmp/whatever").is_ok());
        assert!(std::fs::read_dir(dir.path()).unwrap().next().is_none());
    }
}
