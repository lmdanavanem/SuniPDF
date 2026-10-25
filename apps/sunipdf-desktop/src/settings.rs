//! User preferences are stored atomically in the platform's per-user configuration directory.

use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::PathBuf;
use sunipdf_i18n::Language;

const MAX_SETTINGS_BYTES: usize = 1024 * 1024;

pub(crate) fn absolute_path(value: Option<std::ffi::OsString>) -> Option<PathBuf> {
    value.map(PathBuf::from).filter(|path| path.is_absolute())
}

pub(crate) fn storage_path(root: PathBuf, name: &str) -> PathBuf {
    root.join(if cfg!(any(windows, target_os = "macos")) {
        "SuniPDF"
    } else {
        "sunipdf"
    })
    .join(name)
}

/// macOS keeps per-user settings and recovery data in `~/Library/Application Support`.
#[cfg(target_os = "macos")]
pub(crate) fn application_support() -> Option<PathBuf> {
    absolute_path(std::env::var_os("HOME")).map(|home| home.join("Library/Application Support"))
}

#[cfg(not(any(windows, target_os = "macos")))]
pub(crate) fn xdg_directory(
    configured: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
    fallback: &str,
) -> Option<PathBuf> {
    absolute_path(configured).or_else(|| absolute_path(home).map(|path| path.join(fallback)))
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub language: String,
    pub dark_theme: bool,
    #[serde(deserialize_with = "deserialize_theme_style")]
    pub theme_style: crate::theme::Style,
    /// Missing mode preserves the explicit light/dark preference from older versions.
    #[serde(deserialize_with = "deserialize_theme_mode")]
    pub theme_mode: Option<crate::theme::Mode>,
    pub reduce_transparency: bool,
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
    #[serde(deserialize_with = "deserialize_register_presets")]
    pub register_presets: sunipdf_core::register_presets::RegisterPresetLibrary,
    pub tables: crate::workspace_table::Preferences,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: Language::DEFAULT.code().to_string(),
            dark_theme: false,
            theme_style: crate::theme::Style::Classic,
            theme_mode: None,
            reduce_transparency: false,
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
            register_presets: Default::default(),
            tables: Default::default(),
        }
    }
}

fn deserialize_theme_style<'de, D>(deserializer: D) -> Result<crate::theme::Style, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).unwrap_or_default())
}

fn deserialize_theme_mode<'de, D>(deserializer: D) -> Result<Option<crate::theme::Mode>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).ok())
}

fn deserialize_register_presets<'de, D>(
    deserializer: D,
) -> Result<sunipdf_core::register_presets::RegisterPresetLibrary, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // A malformed optional library must not reset unrelated preferences. The settings
    // reader bounds the whole file; the portable parser also bounds this field.
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::to_vec(&value)
        .ok()
        .and_then(|bytes| {
            sunipdf_core::register_presets::RegisterPresetLibrary::from_json(&bytes).ok()
        })
        .unwrap_or_default())
}

impl Settings {
    pub fn effective_theme_mode(&self) -> crate::theme::Mode {
        self.theme_mode.unwrap_or(if self.dark_theme {
            crate::theme::Mode::Dark
        } else {
            crate::theme::Mode::Light
        })
    }

    pub fn commit_appearance(
        &mut self,
        style: crate::theme::Style,
        mode: crate::theme::Mode,
        reduce_transparency: bool,
        resolved_dark: bool,
        writer: impl FnOnce(&Self) -> std::io::Result<()>,
    ) -> std::io::Result<()> {
        let mut next = self.clone();
        next.theme_style = style;
        next.theme_mode = Some(mode);
        next.reduce_transparency = reduce_transparency;
        next.dark_theme = resolved_dark;
        writer(&next)?;
        *self = next;
        Ok(())
    }

    pub fn language(&self) -> Language {
        Language::from_code(&self.language).unwrap_or(Language::DEFAULT)
    }

    pub fn set_language(&mut self, language: Language) {
        self.language = language.code().to_string();
    }

    pub fn push_recent(&mut self, path: &std::path::Path) {
        #[cfg(windows)]
        let entry = path.to_string_lossy().replace('/', "\\");
        #[cfg(not(windows))]
        let entry = path.to_string_lossy().into_owned();
        #[cfg(windows)]
        let key = entry.to_lowercase();
        #[cfg(windows)]
        self.recent.retain(|r| r.to_lowercase() != key);
        #[cfg(not(windows))]
        self.recent.retain(|r| r != &entry);
        self.recent.insert(0, entry);
        self.recent.truncate(8);
    }

    pub fn path() -> Option<PathBuf> {
        #[cfg(windows)]
        let root = absolute_path(std::env::var_os("APPDATA"));
        #[cfg(target_os = "macos")]
        let root = application_support();
        #[cfg(not(any(windows, target_os = "macos")))]
        let root = xdg_directory(
            std::env::var_os("XDG_CONFIG_HOME"),
            std::env::var_os("HOME"),
            ".config",
        );
        root.map(|root| storage_path(root, "settings.json"))
    }

    pub fn load() -> Self {
        Self::path()
            .and_then(|p| {
                let file = std::fs::File::open(p).ok()?;
                let mut text = String::new();
                file.take((MAX_SETTINGS_BYTES + 1) as u64)
                    .read_to_string(&mut text)
                    .ok()?;
                (text.len() <= MAX_SETTINGS_BYTES).then_some(text)
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

    pub fn commit_register_presets(
        &mut self,
        presets: sunipdf_core::register_presets::RegisterPresetLibrary,
        writer: impl FnOnce(&Self) -> std::io::Result<()>,
    ) -> std::io::Result<()> {
        presets.to_json().map_err(|error| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, error.to_string())
        })?;
        let mut next = self.clone();
        next.register_presets = presets;
        writer(&next)?;
        *self = next;
        Ok(())
    }

    #[cfg_attr(test, allow(dead_code))]
    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = Self::path() else {
            return Ok(());
        };
        self.save_to(&path)
    }

    #[cfg_attr(test, allow(dead_code))]
    pub fn save_appearance(&self) -> std::io::Result<()> {
        let path = Self::path().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "preferences directory is unavailable",
            )
        })?;
        self.save_to(&path)
    }

    fn save_to(&self, path: &std::path::Path) -> std::io::Result<()> {
        let bytes = serde_json::to_vec_pretty(self)?;
        if bytes.len() > MAX_SETTINGS_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "preferences exceed 1 MiB",
            ));
        }
        sunipdf_core::write_atomic(path, &bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_preset_migration_and_invalid_library_keep_unrelated_preferences() {
        for library in [
            "null".to_owned(),
            r#"{"format":"sunipdf-register-filter-presets","version":99,"presets":[]}"#.into(),
            format!("{{\"invalid\":\"{}\"}}", "x".repeat(128 * 1024)),
        ] {
            let settings: Settings = serde_json::from_str(&format!(
                r#"{{"language":"fr-FR","dark_theme":true,"author":"Reviewer","recent":["drawing.pdf"],"register_presets":{library}}}"#
            )).unwrap();
            assert_eq!(settings.language, "fr-FR");
            assert!(settings.dark_theme);
            assert_eq!(settings.author, "Reviewer");
            assert_eq!(settings.recent, vec!["drawing.pdf"]);
            assert!(settings.register_presets.entries().is_empty());
        }
        let previous: Settings =
            serde_json::from_str(r#"{"language":"fr-FR","autosave":false}"#).unwrap();
        assert_eq!(previous.language, "fr-FR");
        assert!(!previous.autosave);
        assert!(previous.register_presets.entries().is_empty());
    }

    #[test]
    fn register_presets_commit_and_reopen_without_losing_preferences_or_failed_changes() {
        use sunipdf_core::{
            annotation_register::{RegisterFilter, RegisterSort},
            register_presets::{RegisterPreset, RegisterPresetLibrary},
        };
        let mut settings = Settings {
            language: "fr-FR".into(),
            author: "Reviewer".into(),
            reduce_transparency: true,
            recent: vec!["drawing.pdf".into()],
            ..Default::default()
        };
        let original = settings.clone();
        let mut library = RegisterPresetLibrary::default();
        library
            .insert(
                RegisterPreset::capture(
                    "Electrical",
                    &RegisterFilter {
                        discipline: "Electrical".into(),
                        custom_sort: Some("Room".into()),
                        ..Default::default()
                    },
                    RegisterSort::Author,
                    true,
                )
                .unwrap(),
            )
            .unwrap();
        assert!(settings
            .commit_register_presets(library.clone(), |_| {
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "offline refusal",
                ))
            })
            .is_err());
        assert_eq!(settings, original);
        let dir =
            std::env::temp_dir().join(format!("sunipdf-register-prefs-{}", std::process::id()));
        let path = dir.join("settings.json");
        settings
            .commit_register_presets(library.clone(), |next| next.save_to(&path))
            .unwrap();
        let reopened: Settings = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(reopened, settings);
        let mut expected = original;
        expected.register_presets = library;
        assert_eq!(settings, expected);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn round_trip_and_defaults() {
        let dir = std::env::temp_dir().join(format!("sunipdf-settings-{}", std::process::id()));
        let path = dir.join("settings.json");
        let mut s = Settings::default();
        assert_eq!(s.language(), Language::English);
        s.set_language(Language::French);
        s.dark_theme = true;
        s.check_updates = false;
        s.save_to(&path).unwrap();
        let stored = std::fs::read(&path).unwrap();
        let oversized = Settings {
            recent: vec!["x".repeat(MAX_SETTINGS_BYTES + 1)],
            ..Default::default()
        };
        assert!(oversized.save_to(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), stored);
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
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    #[cfg(windows)]
    fn recent_windows_paths_ignore_case_and_separator_style() {
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
    }

    #[test]
    #[cfg(not(windows))]
    fn recent_unix_paths_preserve_case_and_separators() {
        let mut settings = Settings::default();
        for i in 0..10 {
            settings.push_recent(std::path::Path::new(&format!("/home/user/{i}.pdf")));
        }
        settings.push_recent(std::path::Path::new("/home/user/Plan.pdf"));
        settings.push_recent(std::path::Path::new("/home/user/plan.pdf"));
        settings.push_recent(std::path::Path::new("/home/user/Plan.pdf"));
        assert_eq!(settings.recent.len(), 8);
        assert_eq!(
            &settings.recent[..2],
            &["/home/user/Plan.pdf", "/home/user/plan.pdf"]
        );
        assert!(settings.recent.iter().all(|path| path.starts_with('/')));
    }

    #[test]
    #[cfg(not(any(windows, target_os = "macos")))]
    fn xdg_paths_require_absolute_roots_and_fall_back_to_home() {
        assert_eq!(
            xdg_directory(
                Some("/custom/config".into()),
                Some("/home/user".into()),
                ".config"
            ),
            Some(PathBuf::from("/custom/config"))
        );
        for invalid in [None, Some("".into()), Some("relative".into())] {
            assert_eq!(
                xdg_directory(invalid, Some("/home/user".into()), ".config"),
                Some(PathBuf::from("/home/user/.config"))
            );
        }
        assert!(xdg_directory(None, None, ".config").is_none());
        assert!(
            xdg_directory(Some("relative".into()), Some("relative".into()), ".config").is_none()
        );
        let home = Some(std::ffi::OsString::from("/home/user"));
        assert_eq!(
            storage_path(
                xdg_directory(None, home.clone(), ".config").unwrap(),
                "settings.json"
            ),
            PathBuf::from("/home/user/.config/sunipdf/settings.json")
        );
        assert_eq!(
            storage_path(
                xdg_directory(None, home, ".local/state").unwrap(),
                "recovery"
            ),
            PathBuf::from("/home/user/.local/state/sunipdf/recovery")
        );
    }
    #[test]
    fn appearance_migrates_legacy_modes_and_tolerates_future_theme_values() {
        use crate::theme::{Mode, Style};
        for dark in [false, true] {
            let settings: Settings = serde_json::from_value(serde_json::json!({
                "language": "fr-FR", "dark_theme": dark, "author": "Reviewer"
            }))
            .unwrap();
            assert_eq!(settings.theme_style, Style::Classic);
            assert_eq!(
                settings.effective_theme_mode(),
                if dark { Mode::Dark } else { Mode::Light }
            );
            assert!(!settings.reduce_transparency);
            for value in [
                serde_json::json!("future_theme"),
                serde_json::json!(42),
                serde_json::json!({"future": true}),
                serde_json::Value::Null,
            ] {
                let future: Settings = serde_json::from_value(serde_json::json!({
                    "language": "fr-FR", "dark_theme": dark, "author": "Reviewer",
                    "theme_style": value, "theme_mode": value
                }))
                .unwrap();
                assert_eq!(future.language(), Language::French);
                assert_eq!(future.author, "Reviewer");
                assert_eq!(future.theme_style, Style::Classic);
                assert_eq!(
                    future.effective_theme_mode(),
                    settings.effective_theme_mode()
                );
            }
        }
    }

    #[test]
    fn appearance_commit_round_trips_and_failure_preserves_all_preferences() {
        use crate::theme::{Mode, Style};
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let mut settings = Settings {
            language: "fr-FR".into(),
            author: "Reviewer".into(),
            ..Default::default()
        };
        settings
            .commit_appearance(Style::LiquidGlass, Mode::System, true, true, |next| {
                next.save_to(&path)
            })
            .unwrap();
        let stored = std::fs::read(&path).unwrap();
        let back: Settings = serde_json::from_slice(&stored).unwrap();
        assert_eq!(back, settings);
        assert_eq!(back.effective_theme_mode(), Mode::System);
        let before = settings.clone();
        let result = settings.commit_appearance(Style::Classic, Mode::Light, false, false, |_| {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "fixture refusal",
            ))
        });
        assert!(result.is_err());
        assert_eq!(settings, before);
        assert_eq!(std::fs::read(&path).unwrap(), stored);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
