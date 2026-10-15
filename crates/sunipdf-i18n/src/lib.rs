//! Internationalisation SuniPDF.
//!
//! Source de vérité : `assets/i18n/fr-FR.json` et `assets/i18n/en-US.json`, embarqués à la
//! compilation. Aucune chaîne d'interface ne doit être écrite en dur ailleurs.
//! Clé absente → la clé elle-même est renvoyée (visible, jamais silencieuse) ; les tests
//! garantissent la parité des deux catalogues.

use std::collections::HashMap;
use std::sync::OnceLock;

const FR_FR: &str = include_str!("../../../assets/i18n/fr-FR.json");
const EN_US: &str = include_str!("../../../assets/i18n/en-US.json");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Language {
    French,
    English,
}

impl Language {
    pub const ALL: [Language; 2] = [Language::French, Language::English];
    pub const DEFAULT: Language = Language::English;

    /// Code BCP 47.
    pub fn code(self) -> &'static str {
        match self {
            Self::French => "fr-FR",
            Self::English => "en-US",
        }
    }

    /// Nom de la langue dans la langue elle-même (jamais traduit).
    pub fn native_name(self) -> &'static str {
        match self {
            Self::French => "Français",
            Self::English => "English",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|l| l.code().eq_ignore_ascii_case(code))
    }

    fn raw(self) -> &'static str {
        match self {
            Self::French => FR_FR,
            Self::English => EN_US,
        }
    }
}

type Catalog = HashMap<String, String>;

fn catalog(language: Language) -> &'static Catalog {
    static FR: OnceLock<Catalog> = OnceLock::new();
    static EN: OnceLock<Catalog> = OnceLock::new();
    let cell = match language {
        Language::French => &FR,
        Language::English => &EN,
    };
    cell.get_or_init(|| serde_json::from_str(language.raw()).expect("catalogue i18n JSON invalide"))
}

/// Traduction d'une clé. Clé inconnue → renvoie la clé (jamais de panique).
pub fn tr(language: Language, key: &str) -> &'static str {
    match catalog(language).get_key_value(key) {
        Some((_, v)) => v.as_str(),
        None => catalog(Language::DEFAULT)
            .get_key_value(key)
            .map(|(_, v)| v.as_str())
            .unwrap_or_else(|| missing(key)),
    }
}

/// Traduction avec substitution de paramètres `{nom}`.
pub fn tr_args(language: Language, key: &str, args: &[(&str, &str)]) -> String {
    let mut text = tr(language, key).to_string();
    for (name, value) in args {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

/// Clés présentes dans le catalogue de la langue donnée.
pub fn keys(language: Language) -> Vec<&'static str> {
    let mut keys: Vec<&str> = catalog(language).keys().map(String::as_str).collect();
    keys.sort_unstable();
    keys
}

fn missing(key: &str) -> &'static str {
    // Fuite volontaire et rare : ne survient que si une clé manque dans les deux catalogues,
    // ce que les tests interdisent.
    Box::leak(key.to_string().into_boxed_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogs_have_identical_key_sets() {
        assert_eq!(keys(Language::French), keys(Language::English));
        assert!(!keys(Language::French).is_empty());
    }

    #[test]
    fn no_empty_values() {
        for lang in Language::ALL {
            for key in keys(lang) {
                assert!(
                    !tr(lang, key).trim().is_empty(),
                    "{} vide en {}",
                    key,
                    lang.code()
                );
            }
        }
    }

    #[test]
    fn placeholders_match_between_languages() {
        for key in keys(Language::French) {
            let holders = |lang| {
                let mut h: Vec<String> = tr(lang, key)
                    .split('{')
                    .skip(1)
                    .filter_map(|s| s.split('}').next().map(str::to_string))
                    .collect();
                h.sort();
                h
            };
            assert_eq!(
                holders(Language::French),
                holders(Language::English),
                "clé {key}"
            );
        }
    }

    #[test]
    fn missing_key_returns_key_itself() {
        assert_eq!(tr(Language::English, "no.such.key"), "no.such.key");
    }

    #[test]
    fn language_codes_round_trip() {
        for lang in Language::ALL {
            assert_eq!(Language::from_code(lang.code()), Some(lang));
        }
        assert_eq!(Language::from_code("fr-fr"), Some(Language::French));
        assert_eq!(Language::from_code("de-DE"), None);
    }

    #[test]
    fn tr_args_substitutes_placeholders() {
        let s = tr_args(
            Language::French,
            "status.page_of",
            &[("current", "2"), ("total", "9")],
        );
        assert!(s.contains('2') && s.contains('9'), "{s}");
    }
}
