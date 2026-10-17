//! Bounded SFNT face selection and library-based subsetting for PDFium.

use crate::EngineError;
use allsorts::binary::read::{ReadArrayCow, ReadScope};
use allsorts::binary::write::{WriteBinary, WriteBuffer};
use allsorts::cff::{Charset, CustomCharset, Operand, Operator, CFF};
use allsorts::error::ParseError;
use allsorts::font::{Font, MatchingPresentation};
use allsorts::font_data::{DynamicFontTableProvider, FontData};
use allsorts::get_name::fontcode_get_name;
use allsorts::subset::{subset, whole_font, CmapTarget, SubsetProfile};
use allsorts::tables::FontTableProvider;
use allsorts::tag;
use std::borrow::Cow;
use std::collections::BTreeSet;
use std::io::Read;
use std::path::Path;

pub(crate) const MAX_FONT_BYTES: usize = 32 * 1024 * 1024;
const MAX_FACES: u32 = 64;
const MAX_TEXT_CHARS: usize = 65_536;
const MAX_SUBSET_GLYPHS: usize = 4096;

pub(crate) struct PreparedFont {
    pub bytes: Vec<u8>,
    pub is_cff: bool,
}

pub(crate) fn read_font(path: &Path) -> Result<Vec<u8>, EngineError> {
    let file = std::fs::File::open(path)?;
    if file.metadata()?.len() > MAX_FONT_BYTES as u64 {
        return Err(EngineError::FontNotEmbeddable);
    }
    let mut bytes = Vec::new();
    file.take(MAX_FONT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_FONT_BYTES {
        return Err(EngineError::FontNotEmbeddable);
    }
    Ok(bytes)
}

fn face_count(bytes: &[u8]) -> Result<u32, EngineError> {
    if bytes.len() < 12 || bytes.len() > MAX_FONT_BYTES {
        return Err(EngineError::FontNotEmbeddable);
    }
    if &bytes[..4] == b"ttcf" {
        let count = u32::from_be_bytes(bytes[8..12].try_into().unwrap());
        if count == 0 || count > MAX_FACES {
            return Err(EngineError::FontNotEmbeddable);
        }
        Ok(count)
    } else if bytes[..4] == [0, 1, 0, 0] || &bytes[..4] == b"true" || &bytes[..4] == b"OTTO" {
        Ok(1)
    } else {
        Err(EngineError::FontNotEmbeddable)
    }
}

fn provider(bytes: &[u8], face_index: u32) -> Result<DynamicFontTableProvider<'_>, EngineError> {
    if face_index >= face_count(bytes)? {
        return Err(EngineError::FontNotEmbeddable);
    }
    let data = ReadScope::new(bytes)
        .read::<FontData<'_>>()
        .map_err(|_| EngineError::FontNotEmbeddable)?;
    data.table_provider(face_index as usize)
        .map_err(|_| EngineError::FontNotEmbeddable)
}

fn permissions(provider: &impl FontTableProvider) -> Result<bool, EngineError> {
    let os2 = provider
        .table_data(tag::OS_2)
        .map_err(|_| EngineError::FontNotEmbeddable)?;
    let flags = match os2 {
        None => 0,
        Some(table) => u16::from_be_bytes(
            table
                .get(8..10)
                .ok_or(EngineError::FontNotEmbeddable)?
                .try_into()
                .unwrap(),
        ),
    };
    crate::fonts::embedding_permissions(flags)
}

fn outline(provider: &impl FontTableProvider) -> Result<bool, EngineError> {
    let cff = provider.has_table(tag::CFF);
    let true_type = provider.has_table(tag::GLYF) && provider.has_table(tag::LOCA);
    if cff == true_type
        || [
            tag::CFF2,
            tag::FVAR,
            tag::GVAR,
            tag::COLR,
            tag::CBDT,
            tag::SBIX,
            tag::SVG,
        ]
        .iter()
        .any(|tag| provider.has_table(*tag))
    {
        return Err(EngineError::FontNotEmbeddable);
    }
    if cff {
        let data = provider
            .read_table_data(tag::CFF)
            .map_err(|_| EngineError::FontNotEmbeddable)?;
        let table = ReadScope::new(&data)
            .read::<CFF<'_>>()
            .map_err(|_| EngineError::FontNotEmbeddable)?;
        identity_cid_collection(&table)?;
    }
    Ok(cff)
}

/// PDFium authors composite Type 1 text with Identity-H character codes equal to glyph indices.
/// Other CID collections need a collection-aware authoring path, which is refused here.
fn identity_cid_collection(table: &CFF<'_>) -> Result<bool, EngineError> {
    let [font] = table.fonts.as_slice() else {
        return Err(EngineError::FontNotEmbeddable);
    };
    if !font.is_cid_keyed() {
        return Ok(false);
    }
    let Some([Operand::Integer(registry), Operand::Integer(ordering), Operand::Integer(0)]) =
        font.top_dict.get(Operator::ROS)
    else {
        return Err(EngineError::FontNotEmbeddable);
    };
    // CFF1 defines 391 standard strings; Adobe and Identity are custom String INDEX entries.
    let custom_string = |sid: i32| {
        usize::try_from(sid)
            .ok()
            .and_then(|sid| sid.checked_sub(391))
            .and_then(|index| table.string_index.read_object(index))
    };
    if custom_string(*registry) != Some(b"Adobe".as_slice())
        || custom_string(*ordering) != Some(b"Identity".as_slice())
    {
        return Err(EngineError::FontNotEmbeddable);
    }
    Ok(true)
}

struct CffTableProvider<'a, P> {
    original: &'a P,
    table: &'a [u8],
}

impl<P: FontTableProvider> FontTableProvider for CffTableProvider<'_, P> {
    fn table_data(&self, tag: u32) -> Result<Option<Cow<'_, [u8]>>, ParseError> {
        if tag == tag::CFF {
            Ok(Some(Cow::Borrowed(self.table)))
        } else {
            self.original.table_data(tag)
        }
    }

    fn has_table(&self, tag: u32) -> bool {
        tag == tag::CFF || self.original.has_table(tag)
    }

    fn table_tags(&self) -> Option<Vec<u32>> {
        self.original.table_tags()
    }
}

fn identity_cid_charset(bytes: &[u8]) -> Result<Vec<u8>, EngineError> {
    let provider = provider(bytes, 0)?;
    let data = provider
        .read_table_data(tag::CFF)
        .map_err(|_| EngineError::FontNotEmbeddable)?;
    let mut table = ReadScope::new(&data)
        .read::<CFF<'_>>()
        .map_err(|_| EngineError::FontNotEmbeddable)?;
    if !identity_cid_collection(&table)? {
        return Ok(bytes.to_vec());
    }
    let font = &mut table.fonts[0];
    let count = u16::try_from(font.char_strings_index.len())
        .ok()
        .filter(|count| *count > 0)
        .ok_or(EngineError::FontNotEmbeddable)?;
    if (1..count).all(|id| font.charset.id_for_glyph(id) == Some(id)) {
        return Ok(bytes.to_vec());
    }
    // Allsorts retains source CIDs when subsetting a CID-keyed CFF. Renumber only the charset,
    // leaving every selected outline, subroutine, cmap, metric and embedding restriction intact.
    font.charset = Charset::Custom(CustomCharset::Format0 {
        glyphs: ReadArrayCow::Owned((1..count).collect()),
    });
    let mut buffer = WriteBuffer::new();
    CFF::write(&mut buffer, &table).map_err(|_| EngineError::FontNotEmbeddable)?;
    let normalized = buffer.into_inner();
    let tags = provider
        .table_tags()
        .ok_or(EngineError::FontNotEmbeddable)?;
    whole_font(
        &CffTableProvider {
            original: &provider,
            table: &normalized,
        },
        &tags,
    )
    .map_err(|_| EngineError::FontNotEmbeddable)
}

pub(crate) fn faces(bytes: &[u8]) -> Result<Vec<(u32, String)>, EngineError> {
    let mut result = Vec::new();
    for index in 0..face_count(bytes)? {
        let face = provider(bytes, index)?;
        if permissions(&face).is_err() || outline(&face).is_err() {
            continue;
        }
        let name = face
            .table_data(tag::NAME)
            .ok()
            .flatten()
            .and_then(|data| fontcode_get_name(&data, 4).ok().flatten())
            .and_then(|name| name.into_string().ok())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| format!("Face {}", index + 1));
        result.push((index, name));
    }
    Ok(result)
}

pub(crate) fn prepare(
    bytes: &[u8],
    face_index: u32,
    text: &str,
) -> Result<PreparedFont, EngineError> {
    if text.chars().count() > MAX_TEXT_CHARS {
        return Err(EngineError::FontNotEmbeddable);
    }
    let provider = provider(bytes, face_index)?;
    let allow_subset = permissions(&provider)?;
    let is_cff = outline(&provider)?;
    if face_count(bytes)? == 1 && !is_cff {
        crate::fonts::check_embeddable(bytes)?;
    }
    let mut font = Font::new(provider).map_err(|_| EngineError::FontNotEmbeddable)?;
    let mut glyphs = BTreeSet::from([0]);
    let mut missing_seen = BTreeSet::new();
    let mut missing = String::new();
    for ch in text.chars().filter(|c| !matches!(c, '\r' | '\n' | '\t')) {
        let (id, _) = font.lookup_glyph_index(ch, MatchingPresentation::NotRequired, None);
        if id == 0 && !ch.is_whitespace() && missing_seen.insert(ch) {
            missing.push(ch);
        }
        glyphs.insert(id);
    }
    if !missing.is_empty() {
        return Err(EngineError::MissingGlyphs(missing));
    }
    // Preserve a small editing repertoire while omitting unrelated large glyph ranges.
    for value in (0x20..=0x024f).chain(0x0370..=0x052f) {
        let ch = char::from_u32(value).unwrap();
        glyphs.insert(
            font.lookup_glyph_index(ch, MatchingPresentation::NotRequired, None)
                .0,
        );
    }
    let provider = &font.font_table_provider;
    let output = if allow_subset {
        if glyphs.len() > MAX_SUBSET_GLYPHS {
            return Err(EngineError::FontNotEmbeddable);
        }
        subset(
            provider,
            &glyphs.into_iter().collect::<Vec<_>>(),
            &SubsetProfile::Minimal,
            CmapTarget::Unicode,
        )
        .map_err(|_| EngineError::FontNotEmbeddable)?
    } else if &bytes[..4] != b"ttcf" {
        bytes.to_vec()
    } else {
        let tags = provider
            .table_tags()
            .ok_or(EngineError::FontNotEmbeddable)?;
        whole_font(provider, &tags).map_err(|_| EngineError::FontNotEmbeddable)?
    };
    if output.len() > MAX_FONT_BYTES {
        return Err(EngineError::FontNotEmbeddable);
    }
    // Do not replace a permitted full standalone face with a larger subset.
    let bytes = if allow_subset && output.len() >= bytes.len() && &bytes[..4] != b"ttcf" {
        bytes.to_vec()
    } else {
        output
    };
    let bytes = if is_cff {
        identity_cid_charset(&bytes)?
    } else {
        bytes
    };
    if bytes.len() > MAX_FONT_BYTES {
        return Err(EngineError::FontNotEmbeddable);
    }
    Ok(PreparedFont { bytes, is_cff })
}

#[cfg(test)]
#[path = "font_embed_tests.rs"]
mod tests;
