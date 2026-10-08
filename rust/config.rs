use crate::{error::Error, i18n::Language, theme::Mode};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub theme: Mode,
    pub language: Language,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            theme: Mode::System,
            language: Language::English,
        }
    }
}
pub fn directory() -> Result<PathBuf, Error> {
    directories::ProjectDirs::from("com", "TinyEasy", "TinyEasy")
        .map(|d| d.config_dir().to_owned())
        .ok_or(Error::Configuration)
}
pub fn load(path: &Path) -> Result<Settings, Error> {
    match std::fs::read(path) {
        Ok(data) => {
            let settings: Settings =
                serde_json::from_slice(&data).map_err(|_| Error::Configuration)?;
            if settings.version != 1 {
                return Err(Error::Configuration);
            }
            Ok(settings)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
        Err(_) => Err(Error::Configuration),
    }
}
pub fn save(path: &Path, settings: &Settings) -> Result<(), Error> {
    // Refuse to silently replace corrupt or future-version preferences.
    load(path)?;
    atomic_write(
        path,
        &serde_json::to_vec(settings).map_err(|_| Error::Configuration)?,
    )
}
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<(), Error> {
    let parent = path.parent().ok_or(Error::File)?;
    std::fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(data)?;
    temp.as_file().sync_all()?;
    temp.persist(path).map_err(|_| Error::File)?;
    Ok(())
}
pub fn legacy_exists() -> bool {
    let Some(base) = directories::BaseDirs::new() else {
        return false;
    };
    // Electron derives userData from package.name unless explicitly overridden.
    [
        "image-compressor-electron",
        "tinyeasy",
        "TinyEasy",
        "tiny-batch-compressor",
    ]
    .iter()
    .any(|name| {
        base.config_dir()
            .join(name)
            .join("tinify-keys.json")
            .exists()
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preferences_restore_and_default_to_system_english() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        assert_eq!(load(&path).unwrap(), Settings::default());
        let settings = Settings {
            theme: Mode::Dark,
            language: Language::Chinese,
            ..Settings::default()
        };
        save(&path, &settings).unwrap();
        assert_eq!(load(&path).unwrap(), settings);
    }
    #[test]
    fn corrupt_preferences_are_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, b"{broken").unwrap();
        assert!(save(&path, &Settings::default()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"{broken");
    }
}
