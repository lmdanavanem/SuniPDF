pub fn embedding_flags(source: &[u8], value: u16) -> Vec<u8> {
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

/// TTC container for licensed test faces; no glyph data is modified or subsetted here.
pub fn collection(fonts: &[&[u8]]) -> Vec<u8> {
    let mut bytes = b"ttcf\0\x01\0\0".to_vec();
    bytes.extend_from_slice(&(fonts.len() as u32).to_be_bytes());
    bytes.resize(12 + fonts.len() * 4, 0);
    for (i, font) in fonts.iter().enumerate() {
        while !bytes.len().is_multiple_of(4) {
            bytes.push(0);
        }
        let offset = bytes.len();
        bytes[12 + i * 4..16 + i * 4].copy_from_slice(&(offset as u32).to_be_bytes());
        let mut face = font.to_vec();
        let tables = u16::from_be_bytes(face[4..6].try_into().unwrap()) as usize;
        for table in 0..tables {
            let at = 12 + table * 16 + 8;
            let original = u32::from_be_bytes(face[at..at + 4].try_into().unwrap());
            face[at..at + 4].copy_from_slice(&(original + offset as u32).to_be_bytes());
        }
        bytes.extend_from_slice(&face);
    }
    bytes
}
