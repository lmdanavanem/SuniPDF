//! Bounded local decoration preferences; image presets retain a path, never image bytes.

use crate::tools::{StampForm, StampKind};
use serde::{Deserialize, Deserializer, Serialize};
use std::{io, path::PathBuf};
use sunipdf_core::StampPosition;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StampPreset {
    pub name: String,
    pub data: PresetData,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PresetData {
    Text {
        text: String,
        size: f32,
        rgb: [u8; 3],
        opacity: f32,
        angle: f32,
        bold: bool,
        position: String,
    },
    Image {
        path: PathBuf,
        width_percent: f32,
        margin: f32,
        opacity: f32,
        angle: f32,
        position: String,
    },
    Numbers {
        template: String,
        size: f32,
        position: String,
    },
    Headers {
        zones: [String; 6],
        size: f32,
        margin: f32,
    },
}

fn line(text: &str) -> bool {
    !text.trim().is_empty() && text.chars().count() <= 256 && !text.chars().any(char::is_control)
}
fn between(value: f32, low: f32, high: f32) -> bool {
    value.is_finite() && (low..=high).contains(&value)
}

impl PresetData {
    pub fn valid(&self) -> bool {
        match self {
            Self::Text {
                text,
                size,
                opacity,
                angle,
                position,
                ..
            } => {
                line(text)
                    && between(*size, 6.0, 300.0)
                    && between(*opacity, 0.05, 1.0)
                    && between(*angle, -180.0, 180.0)
                    && StampPosition::from_id(position).is_some()
            }
            Self::Image {
                path,
                width_percent,
                margin,
                opacity,
                angle,
                position,
            } => {
                !path.as_os_str().is_empty()
                    && path.as_os_str().len() <= 32767
                    && between(*width_percent, 1.0, 100.0)
                    && between(*margin, 0.0, 1000.0)
                    && between(*opacity, 0.05, 1.0)
                    && between(*angle, -180.0, 180.0)
                    && StampPosition::from_id(position).is_some()
            }
            Self::Numbers {
                template,
                size,
                position,
            } => {
                line(template)
                    && between(*size, 4.0, 72.0)
                    && StampPosition::from_id(position).is_some()
            }
            Self::Headers {
                zones,
                size,
                margin,
            } => sunipdf_core::header_footer::HeaderFooter {
                zones: zones.clone(),
                style: sunipdf_core::TextStyle {
                    size_pt: *size,
                    ..Default::default()
                },
                margin_pt: *margin,
            }
            .validate()
            .is_ok(),
        }
    }

    pub fn matches(&self, kind: StampKind) -> bool {
        matches!(
            (self, kind),
            (Self::Text { .. } | Self::Image { .. }, StampKind::Watermark)
                | (
                    Self::Numbers { .. } | Self::Headers { .. },
                    StampKind::PageNumbers
                )
        )
    }

    pub fn capture(form: &StampForm, kind: StampKind) -> Option<Self> {
        let data = match kind {
            StampKind::Watermark if form.image_mode => Self::Image {
                path: form.image_path.clone()?,
                width_percent: form.image_width_percent,
                margin: form.image_margin,
                opacity: form.opacity,
                angle: form.angle,
                position: form.position.id().into(),
            },
            StampKind::Watermark => Self::Text {
                text: form.text.clone(),
                size: form.size,
                rgb: form.rgb,
                opacity: form.opacity,
                angle: form.angle,
                bold: form.bold,
                position: form.position.id().into(),
            },
            StampKind::PageNumbers if form.header_mode => Self::Headers {
                zones: form.headers.zones.clone(),
                size: form.headers.style.size_pt,
                margin: form.headers.margin_pt,
            },
            StampKind::PageNumbers => Self::Numbers {
                template: form.numbers_template.clone(),
                size: form.numbers_size,
                position: form.numbers_position.id().into(),
            },
        };
        data.valid().then_some(data)
    }

    /// Loads a draft only. The worker still validates the image and destination pages on Apply.
    pub fn apply(&self, form: &mut StampForm) -> bool {
        if !self.valid() {
            return false;
        }
        match self {
            Self::Text {
                text,
                size,
                rgb,
                opacity,
                angle,
                bold,
                position,
            } => {
                form.image_mode = false;
                form.text = text.clone();
                form.size = *size;
                form.rgb = *rgb;
                form.opacity = *opacity;
                form.angle = *angle;
                form.bold = *bold;
                form.position = StampPosition::from_id(position).unwrap_or_default();
            }
            Self::Image {
                path,
                width_percent,
                margin,
                opacity,
                angle,
                position,
            } => {
                form.image_mode = true;
                form.image_path = Some(path.clone());
                form.image_width_percent = *width_percent;
                form.image_margin = *margin;
                form.opacity = *opacity;
                form.angle = *angle;
                form.position = StampPosition::from_id(position).unwrap_or_default();
            }
            Self::Numbers {
                template,
                size,
                position,
            } => {
                form.header_mode = false;
                form.numbers_template = template.clone();
                form.numbers_size = *size;
                form.numbers_position = StampPosition::from_id(position).unwrap_or_default();
            }
            Self::Headers {
                zones,
                size,
                margin,
            } => {
                form.header_mode = true;
                form.headers.zones = zones.clone();
                form.headers.style.size_pt = *size;
                form.headers.margin_pt = *margin;
            }
        }
        true
    }
}

#[derive(Default, Clone, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct StampPresets(Vec<StampPreset>);

impl<'de> Deserialize<'de> for StampPresets {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = serde_json::Value::deserialize(deserializer)?;
        let mut store = Self::default();
        if let Some(entries) = raw.as_array() {
            for entry in entries.iter().take(64) {
                if let Ok(preset) = serde_json::from_value::<StampPreset>(entry.clone()) {
                    let _ = store.insert(&preset.name, preset.data);
                }
            }
        }
        Ok(store)
    }
}

impl StampPresets {
    pub fn entries(&self) -> &[StampPreset] {
        &self.0
    }
    pub fn insert(&mut self, name: &str, data: PresetData) -> io::Result<()> {
        let name = name.trim();
        if name.is_empty()
            || name.chars().count() > 64
            || name.chars().any(char::is_control)
            || !data.valid()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid preset name or settings",
            ));
        }
        if self
            .0
            .iter()
            .any(|p| p.name.to_lowercase() == name.to_lowercase())
        {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "a preset with this name already exists",
            ));
        }
        if self.0.len() >= 16 {
            return Err(io::Error::other(
                "at most 16 decoration presets can be stored",
            ));
        }
        self.0.push(StampPreset {
            name: name.into(),
            data,
        });
        Ok(())
    }
    pub fn remove(&mut self, name: &str) -> bool {
        let count = self.0.len();
        self.0.retain(|p| p.name != name);
        self.0.len() != count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    #[test]
    fn decoration_presets_roundtrip_all_modes_without_capturing_document_scope() {
        let mut form = StampForm {
            text: "Draft {n}".into(),
            all_pages: false,
            ..Default::default()
        };
        for mode in 0..4 {
            form.image_mode = mode == 1;
            form.header_mode = mode == 3;
            form.image_path = Some(PathBuf::from("missing-logo.png"));
            form.headers.zones[4] = "{n} / {total}".into();
            let kind = if mode < 2 {
                StampKind::Watermark
            } else {
                StampKind::PageNumbers
            };
            let data = PresetData::capture(&form, kind).unwrap();
            let decoded: PresetData =
                serde_json::from_slice(&serde_json::to_vec(&data).unwrap()).unwrap();
            let mut loaded = StampForm::default();
            assert!(decoded.apply(&mut loaded));
            assert!(loaded.all_pages);
            assert_eq!(PresetData::capture(&loaded, kind).unwrap(), data);
            assert!(data.matches(kind));
        }
    }

    #[test]
    fn invalid_stored_presets_preserve_other_preferences_and_enforce_bounds() {
        let mut store = StampPresets::default();
        let data = PresetData::capture(&StampForm::default(), StampKind::PageNumbers).unwrap();
        store.insert("  Pages  ", data.clone()).unwrap();
        assert!(store.insert("pages", data.clone()).is_err());
        assert!(store.insert(&"n".repeat(65), data.clone()).is_err());
        for index in 1..16 {
            store
                .insert(&format!("Preset {index}"), data.clone())
                .unwrap();
        }
        assert!(store.insert("Too many", data.clone()).is_err());
        let mut values = serde_json::to_value(&store)
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        values.insert(
            0,
            serde_json::json!({"name":"Invalid","data":{"kind":"text","text":"x"}}),
        );
        values.push(serde_json::json!({"name":"extra","data":data}));
        let settings: Settings = serde_json::from_value(
            serde_json::json!({"language":"fr-FR","dark_theme":true,"stamp_presets":values}),
        )
        .unwrap();
        assert_eq!(settings.language(), sunipdf_i18n::Language::French);
        assert!(settings.dark_theme);
        assert_eq!(settings.stamp_presets.entries().len(), 16);
        assert_eq!(settings.stamp_presets.entries()[0].name, "Pages");
        let settings: Settings = serde_json::from_value(
            serde_json::json!({"dark_theme":true,"stamp_presets":{"wrong":"shape"}}),
        )
        .unwrap();
        assert!(settings.dark_theme && settings.stamp_presets.entries().is_empty());
        let old: Settings = serde_json::from_str("{\"language\":\"fr-FR\"}").unwrap();
        assert!(old.stamp_presets.entries().is_empty());
    }

    #[test]
    fn failed_preset_persistence_keeps_active_settings_and_delete_is_exact() {
        let mut settings = Settings::default();
        let before = settings.clone();
        let mut candidate = StampPresets::default();
        candidate
            .insert(
                "Pages",
                PresetData::capture(&StampForm::default(), StampKind::PageNumbers).unwrap(),
            )
            .unwrap();
        assert!(settings
            .commit_stamp_presets(candidate.clone(), |_| Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "generated write refusal"
            )))
            .is_err());
        assert_eq!(settings, before);
        settings
            .commit_stamp_presets(candidate, |next| {
                assert_eq!(next.stamp_presets.entries().len(), 1);
                Ok(())
            })
            .unwrap();
        assert!(!settings.stamp_presets.remove("pages"));
        assert!(settings.stamp_presets.remove("Pages"));
        assert!(settings.stamp_presets.entries().is_empty());
    }

    #[test]
    fn invalid_preset_values_never_replace_the_visible_draft() {
        let mut form = StampForm::default();
        for data in [
            PresetData::Text {
                text: "x".into(),
                size: f32::NAN,
                rgb: [0; 3],
                opacity: 1.0,
                angle: 0.0,
                bold: false,
                position: "center".into(),
            },
            PresetData::Numbers {
                template: "bad\nline".into(),
                size: 10.0,
                position: "center".into(),
            },
            PresetData::Image {
                path: "logo.png".into(),
                width_percent: 0.0,
                margin: 28.0,
                opacity: 1.0,
                angle: 0.0,
                position: "center".into(),
            },
        ] {
            assert!(!data.apply(&mut form));
            assert!(!form.image_mode && !form.header_mode && form.text.is_empty());
        }
    }
}
