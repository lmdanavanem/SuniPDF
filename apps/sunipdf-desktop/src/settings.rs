//! Préférences utilisateur persistées dans `%APPDATA%\SuniPDF\settings.json` (écriture atomique).

use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::PathBuf;
use sunipdf_i18n::Language;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub language: String,
    pub dark_theme: bool,
    pub show_thumbnails: bool,
    pub show_tools: bool,
    /// Défilement continu (toutes les pages empilées) plutôt que page unique.
    pub continuous_view: bool,
    pub view_mode: Option<String>,
    pub autosave: bool,
    /// Fichiers récents, le plus récent en premier.
    pub recent: Vec<String>,
    /// Auteur écrit dans les annotations, réponses et statuts (vide : aucun nom écrit).
    pub author: String,
    pub check_updates: bool,
    pub office_profile: String,
    pub shortcuts: crate::shortcuts::Bindings,
    pub stamp_presets: crate::stamp_presets::StampPresets,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: Language::DEFAULT.code().to_string(),
            dark_theme: false,
            show_thumbnails: true,
            show_tools: true,
            continuous_view: true,
            view_mode: None,
            autosave: true,
            recent: Vec::new(),
            author: String::new(),
            check_updates: true,
            office_profile: "standard".into(),
            shortcuts: Default::default(),
            stamp_presets: Default::default(),
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

    pub fn push_recent(&mut self, path: &std::path::Path) {
        // Chemins Windows : séparateurs et casse non significatifs.
        let entry = path.to_string_lossy().replace('/', "\\");
        let key = entry.to_lowercase();
        self.recent.retain(|r| r.to_lowercase() != key);
        self.recent.insert(0, entry);
        self.recent.truncate(8);
    }

    pub fn path() -> Option<PathBuf> {
        std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("SuniPDF").join("settings.json"))
    }

    pub fn load() -> Self {
        Self::path()
            .and_then(|p| {
                let file = std::fs::File::open(p).ok()?;
                let mut text = String::new();
                file.take(1024 * 1024 + 1).read_to_string(&mut text).ok()?;
                (text.len() <= 1024 * 1024).then_some(text)
            })
            // Tolère le BOM UTF-8 ajouté par certains éditeurs (Bloc-notes, PowerShell 5).
            .and_then(|s| serde_json::from_str(s.trim_start_matches('\u{feff}')).ok())
            .unwrap_or_default()
    }

    pub fn commit_stamp_presets(
        &mut self,
        presets: crate::stamp_presets::StampPresets,
        writer: impl FnOnce(&Self) -> std::io::Result<()>,
    ) -> std::io::Result<()> {
        let mut next = self.clone();
        next.stamp_presets = presets;
        writer(&next)?;
        *self = next;
        Ok(())
    }

    #[cfg_attr(test, allow(dead_code))]
    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = Self::path() else {
            return Ok(());
        };
        sunipdf_core::write_atomic(&path, &serde_json::to_vec_pretty(self)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_defaults() {
        let dir = std::env::temp_dir().join(format!("sunipdf-settings-{}", std::process::id()));
        let path = dir.join("settings.json");
        let mut s = Settings::default();
        assert_eq!(s.language(), Language::English);
        s.set_language(Language::French);
        s.dark_theme = true;
        s.check_updates = false;
        sunipdf_core::write_atomic(&path, &serde_json::to_vec(&s).unwrap()).unwrap();
        let back: Settings =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(back, s);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        let bad: Settings = serde_json::from_str(r#"{"language":"xx-XX"}"#).unwrap();
        assert_eq!(bad.language(), Language::English);
        assert!(bad.show_tools && bad.recent.is_empty());
        // Préférences antérieures à la vue continue : continu par défaut.
        assert!(bad.continuous_view);
        assert!(Settings::default().continuous_view);
        let previous: Settings = serde_json::from_str(r#"{"language":"fr-FR"}"#).unwrap();
        assert_eq!(previous.language(), Language::French);
        assert!(previous.check_updates);
        let with_bom = "\u{feff}{\"dark_theme\":true}";
        let parsed: Settings =
            serde_json::from_str(with_bom.trim_start_matches('\u{feff}')).unwrap();
        assert!(parsed.dark_theme);
        let mut r = Settings::default();
        for i in 0..10 {
            r.push_recent(std::path::Path::new(&format!("c:/f{i}.pdf")));
        }
        r.push_recent(std::path::Path::new(r"C:\F5.PDF"));
        assert_eq!(r.recent.len(), 8);
        assert_eq!(r.recent[0], r"C:\F5.PDF");
        assert_eq!(
            r.recent
                .iter()
                .filter(|x| x.eq_ignore_ascii_case(r"c:\f5.pdf"))
                .count(),
            1
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
