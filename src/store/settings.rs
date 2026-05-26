use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StartupDirectory {
    CurrentDirectory,
    HomeDirectory,
    LastDirectory,
}

impl Default for StartupDirectory {
    fn default() -> Self {
        Self::CurrentDirectory
    }
}

impl StartupDirectory {
    pub const ALL: &'static [StartupDirectory] = &[
        StartupDirectory::CurrentDirectory,
        StartupDirectory::HomeDirectory,
        StartupDirectory::LastDirectory,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            StartupDirectory::CurrentDirectory => "Current Directory",
            StartupDirectory::HomeDirectory => "Home Directory",
            StartupDirectory::LastDirectory => "Last Directory",
        }
    }

    pub fn index(&self) -> usize {
        Self::ALL.iter().position(|d| d == self).unwrap_or(0)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub startup_directory: StartupDirectory,
}

#[derive(Debug)]
pub struct SettingsStore {
    json_path: PathBuf,
    settings: Settings,
}

impl SettingsStore {
    pub fn new() -> Result<Self> {
        let config_dir = dirs::config_dir().context("Failed to get config directory")?;
        let json_path = config_dir.join("fv").join("settings.json");
        Ok(Self {
            json_path,
            settings: Settings::default(),
        })
    }

    pub fn load(&mut self) -> Result<()> {
        match std::fs::read_to_string(&self.json_path) {
            Ok(content) => {
                self.settings =
                    serde_json::from_str(&content).context("Failed to parse settings file")?;
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                self.settings = Settings::default();
                Ok(())
            }
            Err(e) => Err(e).context("Failed to read settings file"),
        }
    }

    fn save(&self) -> Result<()> {
        if let Some(parent) = self.json_path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create settings config directory")?;
        }
        let content =
            serde_json::to_string_pretty(&self.settings).context("Failed to serialize settings")?;
        let tmp_path = self.json_path.with_extension("json.tmp");
        std::fs::write(&tmp_path, content).context("Failed to write settings temp file")?;
        std::fs::rename(&tmp_path, &self.json_path).context("Failed to save settings file")?;
        Ok(())
    }

    pub fn startup_directory(&self) -> &StartupDirectory {
        &self.settings.startup_directory
    }

    pub fn set_startup_directory(&mut self, dir: StartupDirectory) -> Result<()> {
        self.settings.startup_directory = dir;
        self.save()
    }
}
