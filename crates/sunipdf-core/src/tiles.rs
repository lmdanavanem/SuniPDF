//! Géométrie des tuiles de rendu (vue continue, fort zoom). Pur : aucun appel PDFium.
//!
//! Une page rendue à une échelle donnée mesure `page_pixels(..)` pixels. Jusqu'à 4096 px de côté et
//! 8 Mpx elle est rendue d'un seul tenant ; au-delà elle est découpée en tuiles de 1024 px alignées
//! sur une grille ancrée en (0, 0), donc stable pendant le défilement (mêmes clés de cache).

use crate::PageSize;

/// Côté d'une tuile (pixels) pour une page découpée.
pub const TILE_SIDE_PX: u32 = 1024;
/// Une page est rendue d'un seul tenant si ses deux côtés sont <= à ceci...
pub const WHOLE_PAGE_MAX_SIDE_PX: u32 = 4096;
/// ...et si son aire est <= à ceci (8 Mpx = 32 Mio en RGBA).
pub const WHOLE_PAGE_MAX_PIXELS: u32 = 8 * 1024 * 1024;
/// Plafond du côté de la page *entière* à l'échelle demandée (jamais allouée en entier).
pub const MAX_TILED_SIDE_PX: u32 = 131_072;
/// Plafond de l'aire d'une région rendue (100 Mpx = 400 Mo en RGBA).
pub const MAX_REGION_PIXELS: u64 = 100_000_000;

/// Rectangle en pixels dans la page rendue à une échelle donnée, origine en haut à gauche.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl PixelRect {
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(&self) -> u32 {
        self.x.saturating_add(self.width)
    }

    pub fn bottom(&self) -> u32 {
        self.y.saturating_add(self.height)
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub fn intersection(&self, other: &PixelRect) -> Option<PixelRect> {
        let (x0, y0) = (self.x.max(other.x), self.y.max(other.y));
        let (x1, y1) = (
            self.right().min(other.right()),
            self.bottom().min(other.bottom()),
        );
        (x1 > x0 && y1 > y0).then(|| PixelRect::new(x0, y0, x1 - x0, y1 - y0))
    }
}

/// Une tuile à rendre : clé stable (page, échelle en millièmes, région), clé de cache et de dédoublonnage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileJob {
    pub page: usize,
    pub scale_milli: u32,
    pub region: PixelRect,
}

impl TileJob {
    /// Pixels par point PDF.
    pub fn scale(&self) -> f32 {
        self.scale_milli as f32 / 1000.0
    }
}

/// Seuils de découpage, bornés par la taille de texture maximale du GPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileLimits {
    pub tile_side: u32,
    pub whole_max_side: u32,
    pub whole_max_pixels: u32,
}

impl TileLimits {
    /// Une texture ne dépasse jamais `max_texture_side` (le pilote GL panique sinon).
    pub fn for_texture_side(max_texture_side: usize) -> Self {
        let side = max_texture_side.clamp(256, 16_384) as u32;
        Self {
            tile_side: TILE_SIDE_PX.min(side),
            whole_max_side: WHOLE_PAGE_MAX_SIDE_PX.min(side),
            whole_max_pixels: WHOLE_PAGE_MAX_PIXELS,
        }
    }
}

impl Default for TileLimits {
    fn default() -> Self {
        Self::for_texture_side(16_384)
    }
}

/// Échelle arrondie au 1/200 (0,005), en millièmes : évite de relancer les rendus pour des
/// variations imperceptibles (redimensionnement de la fenêtre).
pub fn quantize_milli(scale: f32) -> u32 {
    if !scale.is_finite() || scale <= 0.0 {
        return 5;
    }
    ((scale * 200.0).round().max(1.0) as u32).saturating_mul(5)
}

/// Taille en pixels de la page entière à `scale` (même arrondi que `Document::render_page`).
/// `None` si l'échelle n'est pas finie et positive, ou si un côté vaut 0 ou dépasse
/// [`MAX_TILED_SIDE_PX`].
pub fn page_pixels(size: PageSize, scale: f32) -> Option<(u32, u32)> {
    if !(scale.is_finite() && scale > 0.0) {
        return None;
    }
    let (w, h) = (
        (size.width_pt * scale).round(),
        (size.height_pt * scale).round(),
    );
    let max = MAX_TILED_SIDE_PX as f32;
    (w >= 1.0 && h >= 1.0 && w <= max && h <= max).then_some((w as u32, h as u32))
}

/// Vrai si la page se rend d'un seul tenant (une seule « tuile » = la page entière).
pub fn is_whole(page_px: (u32, u32), limits: &TileLimits) -> bool {
    let (w, h) = page_px;
    w <= limits.whole_max_side
        && h <= limits.whole_max_side
        && u64::from(w) * u64::from(h) <= u64::from(limits.whole_max_pixels)
}

/// Tuiles qui coupent `view` (pixels de la page). Page entière : `[page]` si `view` la coupe.
/// Sinon grille de `tile_side` ancrée en (0, 0), indépendante de `view`, tuiles de bord tronquées.
pub fn plan_tiles(page_px: (u32, u32), view: PixelRect, limits: &TileLimits) -> Vec<PixelRect> {
    let page = PixelRect::new(0, 0, page_px.0, page_px.1);
    let Some(view) = view.intersection(&page) else {
        return Vec::new();
    };
    if is_whole(page_px, limits) {
        return vec![page];
    }
    let t = limits.tile_side.max(1);
    let (tx0, tx1) = (view.x / t, (view.right() - 1) / t);
    let (ty0, ty1) = (view.y / t, (view.bottom() - 1) / t);
    let mut tiles = Vec::with_capacity(((tx1 - tx0 + 1) * (ty1 - ty0 + 1)) as usize);
    for ty in ty0..=ty1 {
        for tx in tx0..=tx1 {
            let (x, y) = (tx * t, ty * t);
            tiles.push(PixelRect::new(
                x,
                y,
                t.min(page_px.0 - x),
                t.min(page_px.1 - y),
            ));
        }
    }
    tiles
}

#[cfg(test)]
mod tests {
    use super::*;

    fn letter() -> PageSize {
        PageSize {
            width_pt: 612.0,
            height_pt: 792.0,
        }
    }

    #[test]
    fn quantize_and_page_pixels() {
        assert_eq!(quantize_milli(1.3333), 1335);
        assert_eq!(quantize_milli(0.1), 100);
        assert_eq!(quantize_milli(0.0), 5);
        assert_eq!(quantize_milli(f32::NAN), 5);
        assert_eq!(page_pixels(letter(), 1.0), Some((612, 792)));
        assert_eq!(page_pixels(letter(), 0.5), Some((306, 396)));
        assert_eq!(page_pixels(letter(), 40.0), Some((24_480, 31_680)));
        for bad in [f32::NAN, 0.0, -1.0, 200.0] {
            assert_eq!(page_pixels(letter(), bad), None, "{bad}");
        }
    }

    #[test]
    fn whole_page_thresholds() {
        let l = TileLimits::default();
        assert!(is_whole((4096, 2048), &l)); // exactement 8 Mpx
        assert!(!is_whole((4097, 100), &l));
        assert!(!is_whole((3000, 3000), &l)); // 9 Mpx
        assert!(is_whole((1224, 1584), &l));
    }

    #[test]
    fn limits_follow_the_gpu() {
        let l = TileLimits::for_texture_side(2048);
        assert_eq!((l.tile_side, l.whole_max_side), (1024, 2048));
        assert_eq!(TileLimits::for_texture_side(512).tile_side, 512);
        let l = TileLimits::for_texture_side(16_384);
        assert_eq!((l.tile_side, l.whole_max_side), (1024, 4096));
        assert_eq!(TileLimits::for_texture_side(10).tile_side, 256);
    }

    #[test]
    fn whole_page_is_one_tile_whatever_the_view() {
        let l = TileLimits::default();
        let page = PixelRect::new(0, 0, 800, 1000);
        assert_eq!(
            plan_tiles((800, 1000), PixelRect::new(10, 900, 5, 5), &l),
            [page]
        );
        assert!(plan_tiles((800, 1000), PixelRect::new(800, 0, 50, 50), &l).is_empty());
    }

    #[test]
    fn grid_is_anchored_and_edge_tiles_are_truncated() {
        let l = TileLimits::default();
        let tiles = plan_tiles((5000, 7000), PixelRect::new(900, 900, 300, 300), &l);
        assert_eq!(
            tiles,
            [
                PixelRect::new(0, 0, 1024, 1024),
                PixelRect::new(1024, 0, 1024, 1024),
                PixelRect::new(0, 1024, 1024, 1024),
                PixelRect::new(1024, 1024, 1024, 1024),
            ]
        );
        let corner = plan_tiles((5000, 7000), PixelRect::new(4990, 6990, 100, 100), &l);
        assert_eq!(corner, [PixelRect::new(4096, 6144, 904, 856)]);
    }

    #[test]
    fn tiles_cover_the_view_without_holes_or_overlap() {
        let l = TileLimits {
            tile_side: 64,
            whole_max_side: 100,
            whole_max_pixels: 10_000,
        };
        let page = (300u32, 230u32);
        for view in [
            PixelRect::new(0, 0, 300, 230),
            PixelRect::new(63, 64, 2, 100),
            PixelRect::new(250, 200, 500, 500),
            PixelRect::new(128, 0, 1, 1),
        ] {
            let tiles = plan_tiles(page, view, &l);
            let clipped = view
                .intersection(&PixelRect::new(0, 0, page.0, page.1))
                .unwrap();
            let mut hits = vec![0u8; (page.0 * page.1) as usize];
            for t in &tiles {
                assert!(t.right() <= page.0 && t.bottom() <= page.1);
                for y in t.y..t.bottom() {
                    for x in t.x..t.right() {
                        hits[(y * page.0 + x) as usize] += 1;
                    }
                }
            }
            for y in clipped.y..clipped.bottom() {
                for x in clipped.x..clipped.right() {
                    assert_eq!(hits[(y * page.0 + x) as usize], 1, "{view:?} ({x},{y})");
                }
            }
            assert!(hits.iter().all(|h| *h <= 1));
        }
    }
}
