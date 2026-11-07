//! Calculator preferences, stored in the `settings` table. The file used by earlier versions
//! (`$XDG_CONFIG_HOME/oxide-calc/settings`, `key=value` lines) is imported once and removed.

use std::path::{Path, PathBuf};

use crate::storage::{Storage, StorageError};

const DARK_THEME: &str = "dark_theme";

/// `None` follows the system colour scheme until the user chooses.
pub fn dark_theme(storage: &Storage) -> Result<Option<bool>, StorageError> {
    Ok(match storage.get_setting(DARK_THEME)?.as_deref() {
        Some("true") => Some(true),
        Some("false") => Some(false),
        _ => None,
    })
}

pub fn set_dark_theme(storage: &Storage, dark: bool) -> Result<(), StorageError> {
    storage.set_setting(DARK_THEME, if dark { "true" } else { "false" })
}

/// Reads a legacy `key=value` file. Invalid lines are ignored.
pub fn parse_legacy(text: &str) -> Option<bool> {
    let mut dark = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some((key, value)) = line.split_once('=')
            && key.trim() == DARK_THEME
        {
            match value.trim() {
                "true" => dark = Some(true),
                "false" => dark = Some(false),
                _ => {}
            }
        }
    }
    dark
}

/// Imports the legacy file once. A value already stored in the database wins; the file is removed.
pub fn import_legacy_file(storage: &Storage, path: &Path) -> Result<(), StorageError> {
    if !path.is_file() {
        return Ok(());
    }
    if let Some(dark) = std::fs::read_to_string(path).ok().and_then(|text| parse_legacy(&text))
        && dark_theme(storage)?.is_none()
    {
        set_dark_theme(storage, dark)?;
    }
    std::fs::remove_file(path)?;
    Ok(())
}

/// `$XDG_CONFIG_HOME/oxide-calc/settings`, falling back to `~/.config`.
pub fn legacy_settings_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("oxide-calc").join("settings")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("oxide-calc-settings-{name}-{nanos}"))
    }

    #[test]
    fn stored_theme_round_trip() {
        let storage = Storage::in_memory().unwrap();
        assert_eq!(dark_theme(&storage).unwrap(), None);
        set_dark_theme(&storage, true).unwrap();
        assert_eq!(dark_theme(&storage).unwrap(), Some(true));
    }

    #[test]
    fn legacy_parse_ignores_invalid_lines() {
        assert_eq!(parse_legacy("sem igual\ndark_theme=talvez\n"), None);
        assert_eq!(parse_legacy("dark_theme = false"), Some(false));
    }

    #[test]
    fn legacy_file_is_imported_once_and_removed() {
        let dir = temp_dir("import");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings");
        std::fs::write(&file, "dark_theme=true\n").unwrap();
        let storage = Storage::in_memory().unwrap();
        import_legacy_file(&storage, &file).unwrap();
        assert_eq!(dark_theme(&storage).unwrap(), Some(true));
        assert!(!file.exists());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn database_value_wins_over_legacy_file() {
        let dir = temp_dir("keep");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings");
        std::fs::write(&file, "dark_theme=true\n").unwrap();
        let storage = Storage::in_memory().unwrap();
        set_dark_theme(&storage, false).unwrap();
        import_legacy_file(&storage, &file).unwrap();
        assert_eq!(dark_theme(&storage).unwrap(), Some(false));
        std::fs::remove_dir_all(dir).ok();
    }
}
