use std::path::Path;

pub fn jpeg(path: &Path, exif: u8) {
    use image::ImageEncoder;
    let pixels = image::RgbImage::from_fn(48, 32, |x, y| {
        image::Rgb(match (x < 24, y < 16) {
            (true, true) => [255, 0, 0],
            (false, true) => [0, 255, 0],
            (true, false) => [0, 0, 255],
            (false, false) => [255, 255, 0],
        })
    });
    let exif = [
        b"II*\0".to_vec(),
        8u32.to_le_bytes().to_vec(),
        1u16.to_le_bytes().to_vec(),
        0x0112u16.to_le_bytes().to_vec(),
        3u16.to_le_bytes().to_vec(),
        1u32.to_le_bytes().to_vec(),
        u16::from(exif).to_le_bytes().to_vec(),
        0u16.to_le_bytes().to_vec(),
        0u32.to_le_bytes().to_vec(),
    ]
    .concat();
    let mut file = std::fs::File::create(path).unwrap();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, 95);
    encoder.set_exif_metadata(exif).unwrap();
    encoder.encode_image(&pixels).unwrap();
}

pub fn transparent_png(path: &Path) {
    image::RgbaImage::from_fn(48, 32, |x, _| {
        image::Rgba(if x < 24 {
            [255, 0, 0, 255]
        } else {
            [0, 255, 0, 0]
        })
    })
    .save(path)
    .unwrap();
}

/// A valid PNG header with no pixel payload, for allocation-free rejection checks.
pub fn png_header(width: u32, height: u32, bit_depth: u8, color_type: u8) -> Vec<u8> {
    fn crc(bytes: &[u8]) -> u32 {
        let mut value = 0xFFFF_FFFFu32;
        for byte in bytes {
            value ^= u32::from(*byte);
            for _ in 0..8 {
                value = if value & 1 == 1 {
                    0xEDB8_8320 ^ (value >> 1)
                } else {
                    value >> 1
                };
            }
        }
        !value
    }
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = b"IHDR".to_vec();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[bit_depth, color_type, 0, 0, 0]);
    for chunk in [header.as_slice(), b"IDAT", b"IEND"] {
        png.extend_from_slice(&(chunk.len() as u32 - 4).to_be_bytes());
        png.extend_from_slice(chunk);
        png.extend_from_slice(&crc(chunk).to_be_bytes());
    }
    png
}

pub fn assert_jpeg_bytes(pdf: &lopdf::Document, page: usize, source: &[u8]) {
    let page = pdf.get_pages()[&(page as u32 + 1)];
    let resources = pdf
        .get_dictionary(page)
        .unwrap()
        .get_deref(b"Resources", pdf)
        .unwrap()
        .as_dict()
        .unwrap();
    let objects = resources
        .get_deref(b"XObject", pdf)
        .unwrap()
        .as_dict()
        .unwrap();
    let images: Vec<_> = objects
        .iter()
        .filter_map(|(_, object)| {
            let object = pdf.dereference(object).unwrap().1;
            let stream = object.as_stream().ok()?;
            (stream.dict.get(b"Subtype").ok()?.as_name().ok()? == b"Image").then_some(stream)
        })
        .collect();
    assert_eq!(images.len(), 1);
    assert_eq!(
        images[0].dict.get(b"Filter").unwrap().as_name().unwrap(),
        b"DCTDecode"
    );
    assert_eq!(images[0].content, source);
}
