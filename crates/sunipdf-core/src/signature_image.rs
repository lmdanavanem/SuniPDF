//! Signature visuelle : préparation d'une image (traits tramés, fond clair rendu transparent,
//! rognage, lecture et écriture PNG). Fonctions pures, sans PDFium.
//!
//! **Aucune valeur cryptographique.** L'image est posée sur la page comme n'importe quelle image
//! (`Document::add_image_data`) : elle ne prouve ni l'identité du signataire ni l'intégrité du
//! document. La signature numérique (certificat) est une autre fonction, non livrée.

use crate::{write_atomic, EngineError};
use image::RgbaImage;
use std::path::Path;

/// Côté maximal accepté au décodage d'un fichier choisi par l'utilisateur.
const MAX_DECODE_SIDE: u32 = 16_384;
/// Largeur de la rampe de luminance (niveaux) qui adoucit le bord du fond retiré.
const RAMP: i32 = 40;
/// Alpha au-dessous duquel un pixel compte pour vide (rognage).
const EMPTY_ALPHA: u8 = 16;

/// Traits (points en pixels, origine en haut à gauche) → image RGBA : tous les pixels portent
/// l'encre `rgb`, seul l'alpha varie (bords anti-crénelés). Un trait d'un seul point donne une
/// pastille ; les points hors image ou non finis sont ignorés.
pub fn rasterize_strokes(
    strokes: &[Vec<(f32, f32)>],
    width: u32,
    height: u32,
    pen_px: f32,
    rgb: (u8, u8, u8),
) -> RgbaImage {
    // ponytail: polyligne brute ; lissage (Chaikin) si les courbes rapides paraissent anguleuses.
    let w = width as usize;
    let mut alpha = vec![0u8; w * height as usize];
    let r = (pen_px / 2.0).max(0.5);
    for stroke in strokes {
        let segments: Vec<((f32, f32), (f32, f32))> = match stroke.as_slice() {
            [] => continue,
            [p] => vec![(*p, *p)],
            pts => pts.windows(2).map(|s| (s[0], s[1])).collect(),
        };
        for (a, b) in segments {
            if ![a.0, a.1, b.0, b.1].iter().all(|v| v.is_finite()) {
                continue;
            }
            let pad = r + 1.0;
            let x_lo = (a.0.min(b.0) - pad).floor().max(0.0);
            let x_hi = (a.0.max(b.0) + pad).ceil().min(width as f32 - 1.0);
            let y_lo = (a.1.min(b.1) - pad).floor().max(0.0);
            let y_hi = (a.1.max(b.1) + pad).ceil().min(height as f32 - 1.0);
            if x_lo > x_hi || y_lo > y_hi {
                continue;
            }
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let len2 = dx * dx + dy * dy;
            for y in y_lo as usize..=y_hi as usize {
                for x in x_lo as usize..=x_hi as usize {
                    let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                    let t = if len2 > 0.0 {
                        (((px - a.0) * dx + (py - a.1) * dy) / len2).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let d = ((px - a.0 - t * dx).powi(2) + (py - a.1 - t * dy).powi(2)).sqrt();
                    let coverage = (r + 0.5 - d).clamp(0.0, 1.0);
                    let i = y * w + x;
                    alpha[i] = alpha[i].max((coverage * 255.0).round() as u8);
                }
            }
        }
    }
    RgbaImage::from_fn(width, height, |x, y| {
        image::Rgba([rgb.0, rgb.1, rgb.2, alpha[y as usize * w + x as usize]])
    })
}

/// Rend transparent le fond clair : alpha = min(alpha, rampe(seuil − luminance)). Les couleurs
/// d'origine sont conservées (encre bleue, par exemple).
pub fn remove_background(img: &RgbaImage, threshold: u8) -> RgbaImage {
    // ponytail: seuil global ; seuil adaptatif si les photos à ombre portée posent problème.
    let mut out = img.clone();
    for p in out.pixels_mut() {
        let luma = (299 * p[0] as i32 + 587 * p[1] as i32 + 114 * p[2] as i32) / 1000;
        let a = ((threshold as i32 - luma) * 255 / RAMP).clamp(0, 255) as u8;
        p[3] = p[3].min(a);
    }
    out
}

/// Vrai si aucun pixel n'est visible (même seuil que `trim`) : poser l'image donnerait un objet
/// invisible.
pub fn is_blank(img: &RgbaImage) -> bool {
    img.pixels().all(|p| p[3] <= EMPTY_ALPHA)
}

/// Rogne à la boîte englobante des pixels visibles, plus `padding` (borné à l'image) ; `None` si
/// l'image est vide.
pub fn trim(img: &RgbaImage, padding: u32) -> Option<RgbaImage> {
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for (x, y, p) in img.enumerate_pixels() {
        if p[3] > EMPTY_ALPHA {
            (x0, y0) = (x0.min(x), y0.min(y));
            (x1, y1) = (x1.max(x), y1.max(y));
        }
    }
    if x0 == u32::MAX {
        return None;
    }
    let (x, y) = (x0.saturating_sub(padding), y0.saturating_sub(padding));
    let w = (x1 + 1).saturating_add(padding).min(img.width()) - x;
    let h = (y1 + 1).saturating_add(padding).min(img.height()) - y;
    Some(image::imageops::crop_imm(img, x, y, w, h).to_image())
}

/// Décode une image choisie par l'utilisateur (format deviné d'après le contenu, 16 384 px de
/// côté et 512 Mio d'allocation au plus, refusés avant tout décodage, orientation EXIF appliquée)
/// et la réduit si un côté dépasse `max_side`.
pub fn load(path: &Path, max_side: u32) -> Result<RgbaImage, EngineError> {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_DECODE_SIDE);
    limits.max_image_height = Some(MAX_DECODE_SIDE);
    let mut img = crate::convert::decode(path, limits)?;
    if img.width().max(img.height()) > max_side {
        // `thumbnail` agrandirait une petite image : réduction seulement.
        img = img.thumbnail(max_side, max_side);
    }
    Ok(img.into_rgba8())
}

/// Écrit un PNG RGBA de façon atomique (fichier temporaire voisin puis renommage).
pub fn save_png(img: &RgbaImage, path: &Path) -> Result<(), EngineError> {
    let mut png = Vec::new();
    image::write_buffer_with_format(
        &mut std::io::Cursor::new(&mut png),
        img.as_raw(),
        img.width(),
        img.height(),
        image::ColorType::Rgba8,
        image::ImageFormat::Png,
    )
    .map_err(|e| crate::convert::image_err(path, e))?;
    write_atomic(path, &png)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rasterize_covers_stroke_and_leaves_rest_transparent() {
        let ink = (0x1E, 0x3A, 0x8A);
        let img = rasterize_strokes(&[vec![(10.0, 10.0), (90.0, 10.0)]], 100, 40, 4.0, ink);
        assert_eq!(img.dimensions(), (100, 40));
        assert_eq!(img.get_pixel(50, 10).0, [0x1E, 0x3A, 0x8A, 255]);
        assert_eq!(img.get_pixel(50, 30)[3], 0);
        assert_eq!(img.get_pixel(5, 10)[3], 0, "avant le début du trait");
        // Un point isolé donne une pastille.
        let dot = rasterize_strokes(&[vec![(20.0, 20.0)]], 40, 40, 6.0, ink);
        assert_eq!(dot.get_pixel(20, 20)[3], 255);
        assert_eq!(dot.get_pixel(30, 20)[3], 0);
        // Trait vide, points hors image, non finis, image vide : aucune panique.
        let weird = rasterize_strokes(
            &[
                vec![],
                vec![(-50.0, -50.0), (500.0, 500.0)],
                vec![(f32::NAN, 1.0), (1.0, 1.0)],
            ],
            30,
            30,
            2.0,
            ink,
        );
        assert_eq!(weird.dimensions(), (30, 30));
        assert!(weird.get_pixel(15, 15)[3] > 0, "diagonale traversante");
        let empty = rasterize_strokes(&[vec![(1.0, 1.0)]], 0, 0, 2.0, ink);
        assert_eq!(empty.dimensions(), (0, 0));
    }

    #[test]
    fn remove_background_soft_threshold() {
        let mut img = RgbaImage::new(4, 1);
        img.put_pixel(0, 0, image::Rgba([250, 250, 250, 255])); // papier
        img.put_pixel(1, 0, image::Rgba([215, 215, 215, 255])); // ombre claire
        img.put_pixel(2, 0, image::Rgba([40, 60, 160, 255])); // encre bleue
        img.put_pixel(3, 0, image::Rgba([40, 40, 40, 0])); // déjà transparent
        let out = remove_background(&img, 200);
        assert_eq!(out.get_pixel(0, 0)[3], 0);
        assert_eq!(out.get_pixel(1, 0)[3], 0);
        assert_eq!(
            out.get_pixel(2, 0).0,
            [40, 60, 160, 255],
            "couleur conservée"
        );
        assert_eq!(out.get_pixel(3, 0)[3], 0);
        // Rampe : une luminance juste sous le seuil n'est que partiellement opaque.
        let mut mid = RgbaImage::new(1, 1);
        mid.put_pixel(0, 0, image::Rgba([180, 180, 180, 255]));
        let a = remove_background(&mid, 200).get_pixel(0, 0)[3];
        assert!(a > 0 && a < 255, "alpha {a}");
    }

    #[test]
    fn trim_bbox_and_empty() {
        let mut img = RgbaImage::new(50, 30);
        img.put_pixel(10, 5, image::Rgba([0, 0, 0, 255]));
        img.put_pixel(20, 12, image::Rgba([0, 0, 0, 255]));
        img.put_pixel(40, 25, image::Rgba([0, 0, 0, 10])); // sous le seuil : ignoré
        let t = trim(&img, 2).unwrap();
        assert_eq!(t.dimensions(), (15, 12)); // x 8..=22, y 3..=14
        assert_eq!(t.get_pixel(2, 2)[3], 255);
        // Marge bornée à l'image.
        let t = trim(&img, 100).unwrap();
        assert_eq!(t.dimensions(), (50, 30));
        assert!(trim(&RgbaImage::new(8, 8), 2).is_none());
        assert!(!is_blank(&img));
        assert!(is_blank(&RgbaImage::new(8, 8)));
        assert!(is_blank(&RgbaImage::from_pixel(
            4,
            4,
            image::Rgba([0, 0, 0, EMPTY_ALPHA])
        )));
    }

    #[test]
    fn png_round_trip_and_downscale() {
        let dir = std::env::temp_dir().join(format!("sunipdf-sig-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let img = RgbaImage::from_fn(300, 100, |x, _| image::Rgba([200, 20, 20, (x % 256) as u8]));
        let path = dir.join("sub").join("signature-1.png");
        save_png(&img, &path).unwrap();
        let back = load(&path, 1600).unwrap();
        assert_eq!(back.dimensions(), (300, 100));
        assert_eq!(back.get_pixel(137, 50).0, [200, 20, 20, 137]);
        assert!(
            !std::fs::read_dir(path.parent().unwrap()).unwrap().any(|e| e
                .unwrap()
                .path()
                .extension()
                .is_some_and(|x| x == "tmp"))
        );
        let small = load(&path, 150).unwrap();
        assert_eq!(small.dimensions(), (150, 50));
        save_png(&RgbaImage::new(40, 20), &dir.join("tiny.png")).unwrap();
        assert_eq!(
            load(&dir.join("tiny.png"), 1600).unwrap().dimensions(),
            (40, 20),
            "jamais agrandie"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// PNG réduit à son en-tête (IHDR, IDAT vide, IEND, CRC corrects) : dimensions et
    /// profondeur annoncées, aucune donnée d'image.
    fn png_header(width: u32, height: u32, bit_depth: u8, color_type: u8) -> Vec<u8> {
        fn crc(bytes: &[u8]) -> u32 {
            let mut c = 0xFFFF_FFFFu32;
            for &b in bytes {
                c ^= u32::from(b);
                for _ in 0..8 {
                    c = if c & 1 == 1 {
                        0xEDB8_8320 ^ (c >> 1)
                    } else {
                        c >> 1
                    };
                }
            }
            !c
        }
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut ihdr = b"IHDR".to_vec();
        ihdr.extend_from_slice(&width.to_be_bytes());
        ihdr.extend_from_slice(&height.to_be_bytes());
        ihdr.extend_from_slice(&[bit_depth, color_type, 0, 0, 0]);
        for chunk in [ihdr.as_slice(), b"IDAT", b"IEND"] {
            png.extend_from_slice(&(chunk.len() as u32 - 4).to_be_bytes());
            png.extend_from_slice(chunk);
            png.extend_from_slice(&crc(chunk).to_be_bytes());
        }
        png
    }

    #[test]
    fn load_refuses_oversized_images_before_decoding() {
        let dir = std::env::temp_dir().join(format!("sunipdf-sig-big-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let refused = |name: &str, bytes: Vec<u8>| {
            let path = dir.join(name);
            std::fs::write(&path, bytes).unwrap();
            match load(&path, 1600) {
                Err(EngineError::Other(m)) => assert!(m.contains("limit"), "{name} : {m}"),
                other => panic!("{name} : {:?}", other.map(|i| i.dimensions())),
            }
        };
        // Pire cas sous la borne de dimensions : 16 384² en RGBA16 = 2 Gio décodés pour un
        // fichier de 33 octets. L'en-tête seul suffit au verdict : rien n'est alloué.
        refused("huge16.png", png_header(16_384, 16_384, 16, 6));
        // Le cas mesuré par la revue : 12 000² en RGBA8 = 549 Mio.
        refused("huge8.png", png_header(12_000, 12_000, 8, 6));
        // Au-delà de 16 384 px de côté, même petite en octets.
        refused("wide.png", png_header(20_000, 8, 8, 0));
        // Le même en-tête à une taille raisonnable passe les bornes et échoue plus loin, au
        // décodage (fichier tronqué) : le refus ci-dessus vient bien des bornes.
        let small = dir.join("small.png");
        std::fs::write(&small, png_header(8, 8, 8, 6)).unwrap();
        match load(&small, 1600) {
            Err(e) => assert!(!e.to_string().contains("limit"), "{e}"),
            Ok(i) => panic!("décodé {:?}", i.dimensions()),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_rejects_missing_and_garbage() {
        let dir = std::env::temp_dir().join(format!("sunipdf-sig-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(matches!(
            load(&dir.join("absent.png"), 100),
            Err(EngineError::Io(_))
        ));
        let garbage = dir.join("texte.png");
        std::fs::write(&garbage, b"ceci n'est pas une image").unwrap();
        assert!(matches!(load(&garbage, 100), Err(EngineError::Other(_))));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
