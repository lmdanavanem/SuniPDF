use super::*;

const TTF: &[u8] = include_bytes!("../tests/fixtures/fonts/Ubuntu-Light.ttf");
const CFF: &[u8] = include_bytes!("../tests/fixtures/fonts/FontAwesome.otf");
const CJK: &[u8] = include_bytes!("../tests/fixtures/fonts/NotoSansJP-Regular.otf");

#[path = "../tests/support/font_collection.rs"]
mod font_container;
use font_container::{collection, embedding_flags as flags};

#[test]
fn subsets_are_deterministic_smaller_and_keep_unicode_and_embedding_flags() {
    let text = "Cafe\u{e9} \u{3a9} \u{41f}\u{440}\u{438}\u{432}\u{435}\u{442}";
    let first = prepare(TTF, 0, text).unwrap();
    let second = prepare(TTF, 0, &text.chars().rev().collect::<String>()).unwrap();
    assert_eq!(first.bytes, second.bytes);
    assert!(!first.is_cff);
    assert!(
        first.bytes.len() < TTF.len() * 9 / 10,
        "{} vs {}",
        first.bytes.len(),
        TTF.len()
    );
    let face = provider(&first.bytes, 0).unwrap();
    assert!(permissions(&face).unwrap());
    let mut font = Font::new(face).unwrap();
    for ch in text.chars().filter(|ch| !ch.is_whitespace()) {
        assert_ne!(
            font.lookup_glyph_index(ch, MatchingPresentation::NotRequired, None)
                .0,
            0
        );
    }
}

#[test]
fn embedding_rights_and_no_subsetting_are_honored() {
    for value in [0x0002, 0x0004, 0x0200, 0x0208, 0x0400] {
        assert!(matches!(
            prepare(&flags(TTF, value), 0, "Text"),
            Err(EngineError::FontNotEmbeddable)
        ));
    }
    for value in [0x0100, 0x0108] {
        let full = flags(TTF, value);
        assert_eq!(prepare(&full, 0, "Text").unwrap().bytes, full);
    }
}

#[test]
fn explicit_ttc_faces_include_cff_and_preserve_full_no_subset_face() {
    let bytes = collection(&[TTF, CFF]);
    let names = faces(&bytes).unwrap();
    assert_eq!(names.iter().map(|f| f.0).collect::<Vec<_>>(), vec![0, 1]);
    assert!(!prepare(&bytes, 0, "Text").unwrap().is_cff);
    assert!(prepare(&bytes, 1, "\u{f000}").unwrap().is_cff);
    assert!(prepare(&bytes, 2, "Text").is_err());
    let restricted = flags(CFF, 2);
    assert_eq!(faces(&collection(&[TTF, &restricted])).unwrap().len(), 1);
    let full = flags(TTF, 0x0100);
    let extracted = prepare(&collection(&[&full, CFF]), 0, "Text").unwrap();
    assert_ne!(&extracted.bytes[..4], b"ttcf");
    let original = Font::new(provider(&full, 0).unwrap()).unwrap();
    let restored = Font::new(provider(&extracted.bytes, 0).unwrap()).unwrap();
    assert_eq!(original.num_glyphs(), restored.num_glyphs());
    assert!(!permissions(&restored.font_table_provider).unwrap());
}

#[test]
fn missing_glyphs_malformed_faces_and_size_limits_are_refused() {
    assert!(matches!(
        prepare(TTF, 0, "\u{65e5}\u{672c}"),
        Err(EngineError::MissingGlyphs(_))
    ));
    for bytes in [b"OTTO".as_slice(), b"ttcf".as_slice(), &[0, 1, 0, 0]] {
        assert!(prepare(bytes, 0, "Text").is_err());
    }
    let mut oversized = b"ttcf\0\x01\0\0".to_vec();
    oversized.extend_from_slice(&65u32.to_be_bytes());
    assert!(faces(&oversized).is_err());
    assert!(prepare(TTF, 0, &"x".repeat(MAX_TEXT_CHARS + 1)).is_err());
}

#[test]
fn cid_cff_charset_matches_pdfium_identity_codes_without_changing_outlines() {
    let prepared = prepare(CJK, 0, "\u{65e5}\u{672c}\u{8a9e}\u{4e2d}\u{6587}").unwrap();
    let face = provider(&prepared.bytes, 0).unwrap();
    let data = face.read_table_data(tag::CFF).unwrap();
    let table = ReadScope::new(&data).read::<CFF<'_>>().unwrap();
    let font = &table.fonts[0];
    assert!(identity_cid_collection(&table).unwrap());
    assert!((1..font.char_strings_index.len()).all(|index| {
        font.charset.id_for_glyph(u16::try_from(index).unwrap()) == Some(index as u16)
    }));
    assert!(prepared.bytes.len() < CJK.len() / 10);

    let full = flags(CJK, 0x0100);
    let normalized = prepare(&full, 0, "\u{65e5}\u{672c}").unwrap();
    let original_provider = provider(&full, 0).unwrap();
    let original_data = original_provider.read_table_data(tag::CFF).unwrap();
    let original = ReadScope::new(&original_data).read::<CFF<'_>>().unwrap();
    let normalized_provider = provider(&normalized.bytes, 0).unwrap();
    let normalized_data = normalized_provider.read_table_data(tag::CFF).unwrap();
    let normalized = ReadScope::new(&normalized_data).read::<CFF<'_>>().unwrap();
    assert_eq!(
        original.fonts[0].char_strings_index.len(),
        normalized.fonts[0].char_strings_index.len(),
        "no-subset embedding must retain every glyph"
    );
    for index in 0..original.fonts[0].char_strings_index.len() {
        assert_eq!(
            original.fonts[0].char_strings_index.read_object(index),
            normalized.fonts[0].char_strings_index.read_object(index),
            "CID normalization must preserve the original glyph program"
        );
    }
    for tag in original_provider.table_tags().unwrap() {
        if !matches!(tag, tag::CFF | tag::HEAD) {
            assert_eq!(
                original_provider.read_table_data(tag).unwrap(),
                normalized_provider.read_table_data(tag).unwrap(),
                "CID normalization changed an unrelated SFNT table"
            );
        }
    }
    assert!(!permissions(&normalized_provider).unwrap());
}

#[test]
fn other_cid_collections_are_refused_before_font_selection() {
    let original = provider(CJK, 0).unwrap();
    let data = original.read_table_data(tag::CFF).unwrap();
    let mut table = ReadScope::new(&data).read::<CFF<'_>>().unwrap();
    let identity = (0..table.string_index.len())
        .find(|index| table.string_index.read_object(*index) == Some(b"Identity".as_slice()))
        .unwrap();
    table.string_index.replace(identity, b"Japan1".to_vec());
    let mut buffer = WriteBuffer::new();
    CFF::write(&mut buffer, &table).unwrap();
    let replacement = buffer.into_inner();
    let bytes = whole_font(
        &CffTableProvider {
            original: &original,
            table: &replacement,
        },
        &original.table_tags().unwrap(),
    )
    .unwrap();
    assert!(faces(&bytes).unwrap().is_empty());
    assert!(matches!(
        prepare(&bytes, 0, "\u{65e5}"),
        Err(EngineError::FontNotEmbeddable)
    ));
}
