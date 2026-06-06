use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// variant 名は serde で settings.json に永続化される文字列表現そのもの。
// `Directory` サフィックスは永続化形式の一部かつ UI ラベルとの対応が取れて読みやすいため、
// enum_variant_names lint を意図的に抑制する。
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum StartupDirectory {
    #[default]
    CurrentDirectory,
    HomeDirectory,
    LastDirectory,
    /// 任意の固定ディレクトリ。保持するパスは入力されたままの文字列で、
    /// `~` 展開や存在確認は起動時の解決（`App::resolve_startup_directory`）で行う。
    SpecificDirectory(String),
}

impl StartupDirectory {
    /// 設定 UI のラジオに並べる選択肢ラベル（表示順）。配列のインデックスが
    /// `index()` および選択位置に対応する。`SpecificDirectory` はパスを持つため
    /// 値の一覧（`&[StartupDirectory]`）ではなくラベル一覧で表現する。
    pub const LABELS: &'static [&'static str] = &[
        "Current Directory",
        "Home Directory",
        "Last Directory",
        "Specific Directory",
    ];

    /// `LABELS` 上で Specific Directory（パスを持つ唯一の選択肢）が占める位置。
    pub const SPECIFIC_INDEX: usize = 3;

    /// この値が `LABELS` 上で占める位置（ラジオの初期選択位置）。
    pub fn index(&self) -> usize {
        match self {
            StartupDirectory::CurrentDirectory => 0,
            StartupDirectory::HomeDirectory => 1,
            StartupDirectory::LastDirectory => 2,
            StartupDirectory::SpecificDirectory(_) => Self::SPECIFIC_INDEX,
        }
    }

    /// `LABELS` 上の選択位置から値を再構成する（`index()` の逆変換）。
    /// Specific の場合のみ `path` を載せ、未知のインデックスは既定（Current）に倒す。
    /// 並び順の知識（`LABELS` / `index` / 本関数）をこのモジュール内に閉じ込めるための入口。
    pub fn from_index(index: usize, path: &str) -> Self {
        match index {
            1 => StartupDirectory::HomeDirectory,
            2 => StartupDirectory::LastDirectory,
            Self::SPECIFIC_INDEX => StartupDirectory::SpecificDirectory(path.to_string()),
            _ => StartupDirectory::CurrentDirectory,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_matches_label_position() {
        assert_eq!(StartupDirectory::CurrentDirectory.index(), 0);
        assert_eq!(StartupDirectory::HomeDirectory.index(), 1);
        assert_eq!(StartupDirectory::LastDirectory.index(), 2);
        assert_eq!(
            StartupDirectory::SpecificDirectory("/anything".to_string()).index(),
            3
        );
        // index() が指す位置に対応するラベルが存在する。
        assert_eq!(StartupDirectory::LABELS.len(), 4);
    }

    #[test]
    fn from_index_is_inverse_of_index() {
        // from_index と index が往復で一致する（並び順の二重管理を防ぐ）。
        for i in 0..StartupDirectory::LABELS.len() {
            assert_eq!(StartupDirectory::from_index(i, "/p").index(), i);
        }
    }

    #[test]
    fn specific_index_points_at_last_label() {
        // SPECIFIC_INDEX は LABELS の末尾を指す。
        assert_eq!(
            StartupDirectory::SPECIFIC_INDEX,
            StartupDirectory::LABELS.len() - 1
        );
        assert!(matches!(
            StartupDirectory::from_index(StartupDirectory::SPECIFIC_INDEX, "/p"),
            StartupDirectory::SpecificDirectory(_)
        ));
    }

    #[test]
    fn specific_directory_round_trips_through_json_with_its_path() {
        let dir = StartupDirectory::SpecificDirectory("/projects/ws".to_string());
        let json = serde_json::to_string(&dir).unwrap();
        let restored: StartupDirectory = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, dir);
    }

    #[test]
    fn legacy_unit_variant_json_still_parses() {
        // 既存 settings.json（ユニットバリアントの文字列表現）が引き続き読める。
        let restored: StartupDirectory = serde_json::from_str("\"HomeDirectory\"").unwrap();
        assert_eq!(restored, StartupDirectory::HomeDirectory);
    }
}
