use super::*;

const TTF: &[u8] = include_bytes!("../tests/fixtures/fonts/Ubuntu-Light.ttf");
const CFF: &[u8] = include_bytes!("../tests/fixtures/fonts/FontAwesome.otf");

fn flags(source: &[u8], value: u16) -> Vec<u8> {
    let mut bytes = source.to_vec();
    let count = u16::from_be_bytes(bytes[4..6].try_into().unwrap()) as usize;
    let record = (0..count)
        .map(|i| 12 + i * 16)
        .find(|&at| &bytes[at..at + 4] == b"OS/2")
        .unwrap();
    let offset = u32::from_be_bytes(bytes[record + 8..record + 12].try_into().unwrap()) as usize;
    bytes[offset + 8..offset + 10].copy_from_slice(&value.to_be_bytes());
    bytes
}

#[path = "../tests/support/font_collection.rs"]
mod font_container;
use font_container::collection;

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
