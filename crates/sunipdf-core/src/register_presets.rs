//! Portable register criteria; document scope and explicit row selection remain local.

use crate::annotation_register::{RegisterFieldFilter, RegisterFilter, RegisterSort};
use crate::EngineError;
use serde::ser::SerializeStruct;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashSet;

pub const MAX_REGISTER_PRESETS: usize = 16;
pub const MAX_REGISTER_PRESET_JSON_BYTES: usize = 128 * 1024;
pub const REGISTER_PRESET_FORMAT: &str = "sunipdf-register-filter-presets";

fn invalid(reason: &str) -> EngineError {
    EngineError::Other(format!("register filter presets: {reason}"))
}

fn bounded_text(value: &str, maximum: usize) -> bool {
    value.len() <= maximum && !value.chars().any(char::is_control)
}

fn valid_name(value: &str) -> bool {
    !value.is_empty() && value.trim() == value && bounded_text(value, 64)
}

/// Literal, portable criteria only. Page and named-zone scope are never serialized here.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisterPresetCriteria {
    pub query: String,
    pub author: String,
    pub modified: String,
    pub review_status: String,
    pub business_status: String,
    pub discipline: String,
    pub kind: String,
    pub count_group: String,
    pub include_replies: bool,
    pub custom_sort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_field: Option<RegisterFieldFilter>,
}

impl RegisterPresetCriteria {
    pub fn validate(&self) -> Result<(), EngineError> {
        if let Some(field) = &self.custom_field {
            field.validate()?;
        }
        if [
            &self.query,
            &self.author,
            &self.modified,
            &self.review_status,
            &self.business_status,
            &self.discipline,
            &self.kind,
            &self.count_group,
        ]
        .into_iter()
        .any(|value| !bounded_text(value, 512))
        {
            return Err(invalid(
                "criteria require at most 512 UTF-8 bytes without control characters",
            ));
        }
        if self
            .custom_sort
            .as_deref()
            .is_some_and(|name| !valid_name(name))
        {
            return Err(invalid(
                "custom sort requires a column name of 1 to 64 UTF-8 bytes",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisterPreset {
    pub name: String,
    pub criteria: RegisterPresetCriteria,
    pub sort: RegisterSort,
    pub descending: bool,
}

impl RegisterPreset {
    pub fn capture(
        name: &str,
        filter: &RegisterFilter,
        sort: RegisterSort,
        descending: bool,
    ) -> Result<Self, EngineError> {
        let preset = Self {
            name: name.to_owned(),
            criteria: RegisterPresetCriteria {
                query: filter.query.clone(),
                author: filter.author.clone(),
                modified: filter.modified.clone(),
                review_status: filter.review_status.clone(),
                business_status: filter.business_status.clone(),
                discipline: filter.discipline.clone(),
                kind: filter.kind.clone(),
                count_group: filter.count_group.clone(),
                include_replies: filter.include_replies,
                custom_sort: filter.custom_sort.clone(),
                custom_field: filter.custom_field.clone(),
            },
            sort,
            descending,
        };
        preset.validate()?;
        Ok(preset)
    }

    pub fn validate(&self) -> Result<(), EngineError> {
        if !valid_name(&self.name) {
            return Err(invalid(
                "names require 1 to 64 UTF-8 bytes without surrounding whitespace or control characters",
            ));
        }
        self.criteria.validate()
    }

    /// Applies only portable criteria. The caller must clear any explicit row selection or edit preview.
    pub fn apply_to(
        &self,
        filter: &mut RegisterFilter,
        sort: &mut RegisterSort,
        descending: &mut bool,
    ) -> Result<(), EngineError> {
        self.validate()?;
        *filter = RegisterFilter {
            query: self.criteria.query.clone(),
            author: self.criteria.author.clone(),
            modified: self.criteria.modified.clone(),
            review_status: self.criteria.review_status.clone(),
            business_status: self.criteria.business_status.clone(),
            discipline: self.criteria.discipline.clone(),
            kind: self.criteria.kind.clone(),
            count_group: self.criteria.count_group.clone(),
            include_replies: self.criteria.include_replies,
            custom_sort: self.criteria.custom_sort.clone(),
            custom_field: self.criteria.custom_field.clone(),
            page: filter.page,
            zone: filter.zone.clone(),
            zone_status: filter.zone_status,
        };
        *sort = self.sort;
        *descending = self.descending;
        Ok(())
    }
}

/// A validated local library. Imports merge atomically and never replace matching names.
/// Name lookup and collisions use Rust's Unicode lowercase conversion without normalization.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RegisterPresetLibrary {
    presets: Vec<RegisterPreset>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LibraryInput {
    format: String,
    version: u32,
    #[serde(deserialize_with = "bounded_presets")]
    presets: Vec<PresetInput>,
}

// Keep key presence separate from Option so version 1 also refuses an explicit null field.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PresetInput {
    name: String,
    criteria: CriteriaInput,
    sort: RegisterSort,
    descending: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CriteriaInput {
    query: String,
    author: String,
    modified: String,
    review_status: String,
    business_status: String,
    discipline: String,
    kind: String,
    count_group: String,
    include_replies: bool,
    custom_sort: Option<String>,
    #[serde(default, deserialize_with = "present_field")]
    custom_field: Option<Option<RegisterFieldFilter>>,
}

fn present_field<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Option<RegisterFieldFilter>>, D::Error> {
    Option::<RegisterFieldFilter>::deserialize(deserializer).map(Some)
}

impl PresetInput {
    fn into_preset(self) -> RegisterPreset {
        RegisterPreset {
            name: self.name,
            sort: self.sort,
            descending: self.descending,
            criteria: RegisterPresetCriteria {
                query: self.criteria.query,
                author: self.criteria.author,
                modified: self.criteria.modified,
                review_status: self.criteria.review_status,
                business_status: self.criteria.business_status,
                discipline: self.criteria.discipline,
                kind: self.criteria.kind,
                count_group: self.criteria.count_group,
                include_replies: self.criteria.include_replies,
                custom_sort: self.criteria.custom_sort,
                custom_field: self.criteria.custom_field.flatten(),
            },
        }
    }
}

fn bounded_presets<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<PresetInput>, D::Error> {
    struct Presets;
    impl<'de> de::Visitor<'de> for Presets {
        type Value = Vec<PresetInput>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("at most 16 distinct register filter presets")
        }

        fn visit_seq<A: de::SeqAccess<'de>>(
            self,
            mut sequence: A,
        ) -> Result<Self::Value, A::Error> {
            let mut presets = Vec::new();
            let mut names = HashSet::new();
            while let Some(preset) = sequence.next_element::<PresetInput>()? {
                if presets.len() == MAX_REGISTER_PRESETS
                    || !names.insert(preset.name.to_lowercase())
                {
                    return Err(de::Error::custom(
                        "duplicate or excessive register filter presets",
                    ));
                }
                presets.push(preset);
            }
            Ok(presets)
        }
    }
    deserializer.deserialize_seq(Presets)
}

impl<'de> Deserialize<'de> for RegisterPresetLibrary {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let input = LibraryInput::deserialize(deserializer)?;
        if input.format != REGISTER_PRESET_FORMAT
            || !matches!(input.version, 1 | 2)
            || (input.version == 1
                && input
                    .presets
                    .iter()
                    .any(|preset| preset.criteria.custom_field.is_some()))
        {
            return Err(de::Error::custom(
                "unsupported register filter preset format or version",
            ));
        }
        let library = Self {
            presets: input
                .presets
                .into_iter()
                .map(PresetInput::into_preset)
                .collect(),
        };
        for preset in &library.presets {
            preset.validate().map_err(de::Error::custom)?;
        }
        library.to_json().map_err(de::Error::custom)?;
        Ok(library)
    }
}

impl Serialize for RegisterPresetLibrary {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut object = serializer.serialize_struct("RegisterPresetLibrary", 3)?;
        object.serialize_field("format", REGISTER_PRESET_FORMAT)?;
        let version = if self
            .presets
            .iter()
            .any(|preset| preset.criteria.custom_field.is_some())
        {
            2u32
        } else {
            1u32
        };
        object.serialize_field("version", &version)?;
        object.serialize_field("presets", &self.presets)?;
        object.end()
    }
}

impl RegisterPresetLibrary {
    pub fn entries(&self) -> &[RegisterPreset] {
        &self.presets
    }

    pub fn get(&self, name: &str) -> Option<&RegisterPreset> {
        let key = name.to_lowercase();
        self.presets
            .iter()
            .find(|preset| preset.name.to_lowercase() == key)
    }

    pub fn insert(&mut self, preset: RegisterPreset) -> Result<(), EngineError> {
        preset.validate()?;
        if self.get(&preset.name).is_some() {
            return Err(invalid("a preset with this name already exists"));
        }
        if self.presets.len() == MAX_REGISTER_PRESETS {
            return Err(invalid("at most 16 presets can be stored"));
        }
        let mut candidate = self.clone();
        candidate.presets.push(preset);
        candidate.to_json()?;
        *self = candidate;
        Ok(())
    }

    pub fn remove(&mut self, name: &str) -> bool {
        let key = name.to_lowercase();
        let before = self.presets.len();
        self.presets
            .retain(|preset| preset.name.to_lowercase() != key);
        before != self.presets.len()
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, EngineError> {
        if bytes.len() > MAX_REGISTER_PRESET_JSON_BYTES {
            return Err(invalid("preset JSON exceeds 128 KiB"));
        }
        let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
        serde_json::from_slice(bytes)
            .map_err(|error| invalid(&format!("invalid preset JSON: {error}")))
    }

    /// Complete validation precedes mutation; collisions, invalid entries and capacity errors retain the library.
    pub fn import_json(&mut self, bytes: &[u8]) -> Result<usize, EngineError> {
        let incoming = Self::from_json(bytes)?;
        let count = incoming.presets.len();
        let mut candidate = self.clone();
        for preset in incoming.presets {
            candidate.insert(preset)?;
        }
        *self = candidate;
        Ok(count)
    }

    /// File callers should publish these bytes with `write_new_copy` to preserve existing destinations.
    pub fn to_json(&self) -> Result<Vec<u8>, EngineError> {
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|error| invalid(&format!("preset serialization failed: {error}")))?;
        if bytes.len() > MAX_REGISTER_PRESET_JSON_BYTES {
            return Err(invalid("preset JSON exceeds 128 KiB"));
        }
        Ok(bytes)
    }
}
