//! Bounded STORE-only OpenXML packages; no ZIP64, compression or arbitrary archive paths.

use crate::EngineError;

pub(super) const MAX_PACKAGE_BYTES: usize = 128 * 1024 * 1024;

#[derive(Default)]
pub(crate) struct Package {
    bytes: Vec<u8>,
    directory: Vec<u8>,
    names: std::collections::HashSet<String>,
}

fn error() -> EngineError {
    EngineError::Other("Office package exceeds its limits or contains an invalid part name".into())
}

fn u16le(out: &mut Vec<u8>, n: u16) {
    out.extend_from_slice(&n.to_le_bytes());
}
fn u32le(out: &mut Vec<u8>, n: u32) {
    out.extend_from_slice(&n.to_le_bytes());
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

impl Package {
    pub(crate) fn part(&mut self, name: &str, bytes: &[u8]) -> Result<(), EngineError> {
        if self.names.len() >= 512
            || name.len() > 240
            || name.is_empty()
            || !name.is_ascii()
            || name.starts_with('/')
            || name
                .split('/')
                .any(|p| p.is_empty() || matches!(p, "." | ".."))
            || name.bytes().any(|b| {
                b < 32 || matches!(b, b'\\' | b':' | b'"' | b'<' | b'>' | b'|' | b'?' | b'*')
            })
            || !self.names.insert(name.to_ascii_lowercase())
            || self
                .bytes
                .len()
                .checked_add(bytes.len() + 76 + 2 * name.len())
                .and_then(|n| n.checked_add(self.directory.len() + 22))
                .is_none_or(|n| n > MAX_PACKAGE_BYTES)
        {
            return Err(error());
        }
        let crc = crc32(bytes);
        let size = u32::try_from(bytes.len()).map_err(|_| error())?;
        let offset = u32::try_from(self.bytes.len()).map_err(|_| error())?;
        u32le(&mut self.bytes, 0x04034b50);
        for n in [20, 0, 0, 0, 33] {
            u16le(&mut self.bytes, n);
        }
        for n in [crc, size, size] {
            u32le(&mut self.bytes, n);
        }
        u16le(&mut self.bytes, name.len() as u16);
        u16le(&mut self.bytes, 0);
        self.bytes.extend_from_slice(name.as_bytes());
        self.bytes.extend_from_slice(bytes);
        u32le(&mut self.directory, 0x02014b50);
        for n in [20, 20, 0, 0, 0, 33] {
            u16le(&mut self.directory, n);
        }
        for n in [crc, size, size] {
            u32le(&mut self.directory, n);
        }
        for n in [name.len() as u16, 0, 0, 0, 0] {
            u16le(&mut self.directory, n);
        }
        u32le(&mut self.directory, 0);
        u32le(&mut self.directory, offset);
        self.directory.extend_from_slice(name.as_bytes());
        Ok(())
    }

    pub(crate) fn finish(mut self) -> Result<Vec<u8>, EngineError> {
        if self.names.is_empty() {
            return Err(error());
        }
        let offset = self.bytes.len() as u32;
        let size = self.directory.len() as u32;
        self.bytes.append(&mut self.directory);
        u32le(&mut self.bytes, 0x06054b50);
        for n in [0, 0, self.names.len() as u16, self.names.len() as u16] {
            u16le(&mut self.bytes, n);
        }
        u32le(&mut self.bytes, size);
        u32le(&mut self.bytes, offset);
        u16le(&mut self.bytes, 0);
        if self.bytes.len() > MAX_PACKAGE_BYTES {
            return Err(error());
        }
        Ok(self.bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crc_and_zip_headers_match_the_format_and_unsafe_paths_are_refused() {
        assert_eq!(crc32(b"123456789"), 0xcbf43926);
        let mut archive = Package::default();
        archive.part("word/document.xml", b"hello").unwrap();
        let bytes = archive.finish().unwrap();
        assert_eq!(&bytes[..4], b"PK\x03\x04");
        assert_eq!(&bytes[bytes.len() - 22..bytes.len() - 18], b"PK\x05\x06");
        for bad in [
            "../outside",
            "/root",
            "a\\b",
            "C:/file",
            "a/../b",
            "a//b",
            "",
        ] {
            assert!(Package::default().part(bad, b"x").is_err());
        }
        let mut archive = Package::default();
        archive.part("a", b"x").unwrap();
        assert!(archive.part("A", b"y").is_err());
    }
}
