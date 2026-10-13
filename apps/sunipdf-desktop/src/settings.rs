//! Préférences utilisateur persistées dans `%APPDATA%\SuniPDF\settings.json` (écriture atomique).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use sunipdf_i18n::Language;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub language: String,
    pub dark_theme: bool,
    pub show_thumbnails: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: Language::DEFAULT.code().to_string(),
            dark_theme: false,
            show_thumbnails: true,
        }
    }
}

impl Settings {
    pub fn language(&self) -> Language {
        Language::from_code(&self.language).unwrap_or(Language::DEFAULT)
    }

    pub fn set_language(&mut self, language: Language) {
        self.language = language.code().to_string();
    }

    pub fn path() -> Option<PathBuf> {
        std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("SuniPDF").join("settings.json"))
    }

    pub fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = Self::path() else {
            return Ok(());
        };
        save_atomic(&path, &serde_json::to_vec_pretty(self)?)
    }
}

/// Écrit dans un fichier temporaire voisin puis renomme : jamais de fichier tronqué.
pub fn save_atomic(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_defaults() {
        let dir = std::env::temp_dir().join(format!("sunipdf-settings-{}", std::process::id()));
        let path = dir.join("settings.json");
        let mut s = Settings::default();
        assert_eq!(s.language(), Language::French);
        s.set_language(Language::English);
        s.dark_theme = true;
        save_atomic(&path, &serde_json::to_vec(&s).unwrap()).unwrap();
        let back: Settings =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(back, s);
        assert!(!path.with_extension("json.tmp").exists());
        let bad: Settings = serde_json::from_str(r#"{"language":"xx-XX"}"#).unwrap();
        assert_eq!(bad.language(), Language::French);
        let _ = std::fs::remove_dir_all(dir);
    }
}
