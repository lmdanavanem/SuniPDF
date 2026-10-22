//! Bounded raster overlays of two explicit local PDF pages. Neither PDF is modified.
//!
//! Coordinates here are displayed points with a top-left origin and Y increasing downwards,
//! unlike the bottom-left annotation coordinates. The revision is mapped onto the base page;
//! anything outside that page is clipped. Raster alignment is not a vector comparison.

use crate::local_review::{check_permission, digest, read_pdf};
use crate::{Document, Engine, EngineError, RenderedPage};
use image::ImageEncoder;
use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub const DEFAULT_LONG_SIDE: u32 = 1536;
pub const MAX_LONG_SIDE: u32 = 2048;
const MAX_COORDINATE: f64 = 1_000_000.0;
const MIN_ANCHOR_DISTANCE: f64 = 0.001;

fn invalid(message: &str) -> EngineError {
    EngineError::Other(message.into())
}

fn check_cancel(cancel: &AtomicBool) -> Result<(), EngineError> {
    if cancel.load(Ordering::Relaxed) {
        Err(EngineError::Cancelled)
    } else {
        Ok(())
    }
}

/// Uniform scale and clockwise rotation around the revision's top-left origin, then translation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct OverlayTransform {
    pub translation: [f64; 2],
    pub rotation_degrees: f64,
    pub scale: f64,
}

impl Default for OverlayTransform {
    fn default() -> Self {
        Self {
            translation: [0.0; 2],
            rotation_degrees: 0.0,
            scale: 1.0,
        }
    }
}

impl OverlayTransform {
    pub fn validate(&self) -> Result<(), EngineError> {
        if self
            .translation
            .iter()
            .any(|v| !v.is_finite() || v.abs() > MAX_COORDINATE)
            || !self.rotation_degrees.is_finite()
            || self.rotation_degrees.abs() > 360.0
            || !self.scale.is_finite()
            || !(0.01..=100.0).contains(&self.scale)
        {
            return Err(invalid("Overlay requires finite translation within 1000000 pt, rotation within 360 degrees and scale from 0.01 to 100"));
        }
        Ok(())
    }

    pub fn apply(&self, point: [f64; 2]) -> [f64; 2] {
        let (sin, cos) = self.rotation_degrees.to_radians().sin_cos();
        [
            self.translation[0] + self.scale * (cos * point[0] - sin * point[1]),
            self.translation[1] + self.scale * (sin * point[0] + cos * point[1]),
        ]
    }

    fn inverse_linear(&self) -> [f64; 2] {
        let (sin, cos) = self.rotation_degrees.to_radians().sin_cos();
        [cos / self.scale, sin / self.scale]
    }

    #[cfg(test)]
    fn inverse(&self, point: [f64; 2]) -> [f64; 2] {
        let [cos, sin] = self.inverse_linear();
        let x = point[0] - self.translation[0];
        let y = point[1] - self.translation[1];
        [cos * x + sin * y, -sin * x + cos * y]
    }

    /// Two corresponding pairs determine a similarity transform; no shear or reflection.
    /// Page bounds are checked when preparing the overlay.
    pub fn from_anchors(base: [[f64; 2]; 2], revision: [[f64; 2]; 2]) -> Result<Self, EngineError> {
        for value in base.iter().chain(revision.iter()).flatten() {
            if !value.is_finite() || !(0.0..=MAX_COORDINATE).contains(value) {
                return Err(invalid(
                    "Overlay anchors must be finite nonnegative displayed points",
                ));
            }
        }
        let a = [base[1][0] - base[0][0], base[1][1] - base[0][1]];
        let b = [
            revision[1][0] - revision[0][0],
            revision[1][1] - revision[0][1],
        ];
        let a_length = a[0].hypot(a[1]);
        let b_length = b[0].hypot(b[1]);
        if a_length < MIN_ANCHOR_DISTANCE || b_length < MIN_ANCHOR_DISTANCE {
            return Err(invalid(
                "Each overlay anchor pair must contain two distinct points",
            ));
        }
        let mut transform = Self {
            translation: [0.0; 2],
            rotation_degrees: (b[0] * a[1] - b[1] * a[0])
                .atan2(b[0] * a[0] + b[1] * a[1])
                .to_degrees(),
            scale: a_length / b_length,
        };
        let mapped = transform.apply(revision[0]);
        transform.translation = [base[0][0] - mapped[0], base[0][1] - mapped[1]];
        transform.validate()?;
        Ok(transform)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum OverlayAlignment {
    Manual {
        transform: OverlayTransform,
    },
    TwoPoints {
        base: [[f64; 2]; 2],
        revision: [[f64; 2]; 2],
    },
}

impl Default for OverlayAlignment {
    fn default() -> Self {
        Self::Manual {
            transform: OverlayTransform::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OverlayOptions {
    /// Physical one-based page numbers.
    pub base_page: usize,
    pub revision_page: usize,
    pub alignment: OverlayAlignment,
    /// Revision cyan density; the base magenta density remains 1.
    pub opacity: f32,
    /// Independent upper bound for each source raster and for the base canvas.
    pub long_side: u32,
}

impl Default for OverlayOptions {
    fn default() -> Self {
        Self {
            base_page: 1,
            revision_page: 1,
            alignment: OverlayAlignment::default(),
            opacity: 0.65,
            long_side: DEFAULT_LONG_SIDE,
        }
    }
}

impl OverlayOptions {
    pub fn validate(&self) -> Result<(), EngineError> {
        if self.base_page == 0 || self.revision_page == 0 {
            return Err(invalid("Overlay page numbers start at 1"));
        }
        if !(64..=MAX_LONG_SIDE).contains(&self.long_side)
            || !self.opacity.is_finite()
            || !(0.0..=1.0).contains(&self.opacity)
        {
            return Err(invalid(
                "Overlay requires a raster side from 64 to 2048 and opacity from 0 to 1",
            ));
        }
        self.transform().map(|_| ())
    }

    fn transform(&self) -> Result<OverlayTransform, EngineError> {
        match self.alignment {
            OverlayAlignment::Manual { transform } => {
                transform.validate()?;
                Ok(transform)
            }
            OverlayAlignment::TwoPoints { base, revision } => {
                OverlayTransform::from_anchors(base, revision)
            }
        }
    }
}

/// Passwords are used only while opening the selected files; previews never retain them.
#[derive(Default, Clone, Copy)]
pub struct OverlayPasswords<'a> {
    pub base: Option<&'a str>,
    pub revision: Option<&'a str>,
}

#[derive(Clone, Debug, Serialize)]
struct Source {
    path: PathBuf,
    source_sha256: String,
    page: usize,
    page_count: usize,
    display_width_pt: f64,
    display_height_pt: f64,
    crop_box_user_pt: [f32; 4],
    rotation_degrees: u16,
    raster_width: u32,
    raster_height: u32,
}

impl Source {
    fn validate_anchors(&self, points: [[f64; 2]; 2]) -> Result<(), EngineError> {
        if points.iter().any(|p| {
            p[0] < 0.0
                || p[1] < 0.0
                || p[0] > self.display_width_pt
                || p[1] > self.display_height_pt
        }) {
            return Err(invalid(
                "Overlay anchors must lie inside their displayed page",
            ));
        }
        Ok(())
    }
}

/// An immutable preview bound to two exact source files. Use export methods for fresh outputs.
#[derive(Clone)]
pub struct OverlayPreview {
    base: Source,
    revision: Source,
    options: OverlayOptions,
    transform: OverlayTransform,
    base_image: RenderedPage,
    revision_image: RenderedPage,
    image: RenderedPage,
}

impl std::fmt::Debug for OverlayPreview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OverlayPreview")
            .field("base", &self.base)
            .field("revision", &self.revision)
            .field("options", &self.options)
            .field("transform", &self.transform)
            .finish_non_exhaustive()
    }
}

impl OverlayPreview {
    pub fn base_image(&self) -> &RenderedPage {
        &self.base_image
    }
    pub fn revision_image(&self) -> &RenderedPage {
        &self.revision_image
    }
    pub fn image(&self) -> &RenderedPage {
        &self.image
    }
    pub fn options(&self) -> &OverlayOptions {
        &self.options
    }
    pub fn transform(&self) -> OverlayTransform {
        self.transform
    }

    /// Display metadata for the prepared snapshot. Export methods additionally revalidate sources.
    pub fn report(&self) -> Value {
        json!({
            "schema": "sunipdf-revision-overlay", "version": 1,
            "base": self.base, "revision": self.revision,
            "coordinates": "display_top_left_points",
            "transform_direction": "revision_to_base",
            "rotation_direction": "clockwise",
            "transform": self.transform, "alignment": self.options.alignment,
            "canvas": {"width": self.image.width, "height": self.image.height, "page": "base", "clip_to_base": true},
            "colors": {"base": "#ff00ff", "revision": "#00ffff"},
            "opacity_base": 1.0, "opacity_revision": self.options.opacity,
            "long_side": self.options.long_side, "sampling": "bilinear",
            "composition": "multiply_luminance_tints_on_white",
            "limitations": ["raster_preview", "base_page_clipping", "independently_bounded_source_rasters", "no_automatic_alignment"],
        })
    }

    pub fn revalidate_sources(&self, cancel: &AtomicBool) -> Result<(), EngineError> {
        for source in [&self.base, &self.revision] {
            check_cancel(cancel)?;
            if digest(&read_pdf(&source.path)?) != source.source_sha256 {
                return Err(invalid(
                    "An overlay source changed; prepare the overlay again",
                ));
            }
        }
        check_cancel(cancel)
    }

    pub fn export_json(&self, cancel: &AtomicBool) -> Result<Value, EngineError> {
        self.revalidate_sources(cancel)?;
        Ok(self.report())
    }

    pub fn export_png(&self, cancel: &AtomicBool) -> Result<Vec<u8>, EngineError> {
        self.revalidate_sources(cancel)?;
        let mut bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut bytes)
            .write_image(
                &self.image.rgba,
                self.image.width,
                self.image.height,
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|error| invalid(&format!("Unable to encode overlay PNG: {error}")))?;
        self.revalidate_sources(cancel)?;
        Ok(bytes)
    }
}

fn render_source(
    doc: &Document<'_>,
    path: PathBuf,
    hash: String,
    page: usize,
    long_side: u32,
) -> Result<(Source, RenderedPage), EngineError> {
    let size = doc.page_size(page - 1)?;
    let width = f64::from(size.width_pt);
    let height = f64::from(size.height_pt);
    if [width, height]
        .iter()
        .any(|v| !v.is_finite() || *v <= 0.0 || *v > MAX_COORDINATE)
    {
        return Err(EngineError::RenderTooLarge);
    }
    let geometry = doc.page_geometry(page - 1)?;
    let image = doc.render_page(
        page - 1,
        long_side as f32 / size.width_pt.max(size.height_pt),
    )?;
    if image.width == 0
        || image.height == 0
        || image.width > long_side
        || image.height > long_side
        || image.rgba.len() != image.width as usize * image.height as usize * 4
    {
        return Err(EngineError::RenderTooLarge);
    }
    let bbox = geometry.bbox;
    Ok((
        Source {
            path,
            source_sha256: hash,
            page,
            page_count: doc.page_count(),
            display_width_pt: width,
            display_height_pt: height,
            crop_box_user_pt: [bbox.left, bbox.bottom, bbox.right, bbox.top],
            rotation_degrees: geometry.rotation.degrees(),
            raster_width: image.width,
            raster_height: image.height,
        },
        image,
    ))
}

/// Open, permission-check and render both sources independently before compositing.
/// Cancellation is observed between native operations and throughout raster composition.
pub fn prepare_overlay(
    engine: &Engine,
    base: &Path,
    revision: &Path,
    options: OverlayOptions,
    passwords: OverlayPasswords<'_>,
    cancel: &AtomicBool,
) -> Result<OverlayPreview, EngineError> {
    options.validate()?;
    check_cancel(cancel)?;
    // Canonical paths keep later validation tied to the same selection if the working directory changes.
    let base_path = base.canonicalize()?;
    let revision_path = revision.canonicalize()?;
    let base_bytes = read_pdf(&base_path)?;
    check_cancel(cancel)?;
    let revision_bytes = read_pdf(&revision_path)?;
    let base_hash = digest(&base_bytes);
    let revision_hash = digest(&revision_bytes);
    check_cancel(cancel)?;
    let base_doc = engine.open_bytes(base_bytes, passwords.base, &base_path)?;
    check_permission(&base_doc)?;
    check_cancel(cancel)?;
    let revision_doc = engine.open_bytes(revision_bytes, passwords.revision, &revision_path)?;
    check_permission(&revision_doc)?;
    check_cancel(cancel)?;
    let (base, base_image) = render_source(
        &base_doc,
        base_path,
        base_hash,
        options.base_page,
        options.long_side,
    )?;
    check_cancel(cancel)?;
    let (revision, revision_image) = render_source(
        &revision_doc,
        revision_path,
        revision_hash,
        options.revision_page,
        options.long_side,
    )?;
    // Native source documents are no longer needed; release their buffers before compositing.
    drop(base_doc);
    drop(revision_doc);
    check_cancel(cancel)?;
    if let OverlayAlignment::TwoPoints {
        base: a,
        revision: b,
    } = options.alignment
    {
        base.validate_anchors(a)?;
        revision.validate_anchors(b)?;
    }
    let transform = options.transform()?;
    let image = composite(
        &base_image,
        &revision_image,
        [base.display_width_pt, base.display_height_pt],
        [revision.display_width_pt, revision.display_height_pt],
        transform,
        options.opacity,
        cancel,
    )?;
    let preview = OverlayPreview {
        base,
        revision,
        options,
        transform,
        base_image,
        revision_image,
        image,
    };
    preview.revalidate_sources(cancel)?;
    Ok(preview)
}

fn luminance(image: &RenderedPage, x: u32, y: u32) -> f64 {
    let i = ((y * image.width + x) * 4) as usize;
    let p = &image.rgba[i..i + 4];
    let gray =
        (299.0 * f64::from(p[0]) + 587.0 * f64::from(p[1]) + 114.0 * f64::from(p[2])) / 1000.0;
    let alpha = f64::from(p[3]) / 255.0;
    (gray * alpha + 255.0 * (1.0 - alpha)) / 255.0
}

fn sample(image: &RenderedPage, size: [f64; 2], point: [f64; 2]) -> f64 {
    if point[0] < 0.0 || point[1] < 0.0 || point[0] >= size[0] || point[1] >= size[1] {
        return 1.0;
    }
    let x =
        (point[0] * f64::from(image.width) / size[0] - 0.5).clamp(0.0, f64::from(image.width - 1));
    let y = (point[1] * f64::from(image.height) / size[1] - 0.5)
        .clamp(0.0, f64::from(image.height - 1));
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(image.width - 1);
    let y1 = (y0 + 1).min(image.height - 1);
    let dx = x - f64::from(x0);
    let dy = y - f64::from(y0);
    let top = luminance(image, x0, y0) * (1.0 - dx) + luminance(image, x1, y0) * dx;
    let bottom = luminance(image, x0, y1) * (1.0 - dx) + luminance(image, x1, y1) * dx;
    top * (1.0 - dy) + bottom * dy
}

fn composite(
    base: &RenderedPage,
    revision: &RenderedPage,
    base_size: [f64; 2],
    revision_size: [f64; 2],
    transform: OverlayTransform,
    opacity: f32,
    cancel: &AtomicBool,
) -> Result<RenderedPage, EngineError> {
    let mut rgba = Vec::with_capacity(base.rgba.len());
    // Trigonometry is invariant across the raster; only the point mapping runs per pixel.
    let [cos, sin] = transform.inverse_linear();
    let step_x = base_size[0] / f64::from(base.width);
    let step_y = base_size[1] / f64::from(base.height);
    for y in 0..base.height {
        if y.is_multiple_of(32) {
            check_cancel(cancel)?;
        }
        let py = (f64::from(y) + 0.5) * step_y - transform.translation[1];
        for x in 0..base.width {
            let px = (f64::from(x) + 0.5) * step_x - transform.translation[0];
            let revision_point = [cos * px + sin * py, -sin * px + cos * py];
            let a = luminance(base, x, y);
            let b = sample(revision, revision_size, revision_point);
            let cyan_red = 1.0 - (1.0 - b) * f64::from(opacity);
            rgba.extend_from_slice(&[
                (cyan_red * 255.0).round() as u8,
                (a * 255.0).round() as u8,
                255,
                255,
            ]);
        }
    }
    check_cancel(cancel)?;
    Ok(RenderedPage {
        page_index: base.page_index,
        width: base.width,
        height: base.height,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raster(width: u32, height: u32, black: &[(u32, u32)]) -> RenderedPage {
        let mut rgba = vec![255; (width * height * 4) as usize];
        for &(x, y) in black {
            let i = ((y * width + x) * 4) as usize;
            rgba[i..i + 3].fill(0);
        }
        RenderedPage {
            page_index: 0,
            width,
            height,
            rgba,
        }
    }

    fn pixel(image: &RenderedPage, x: u32, y: u32) -> &[u8] {
        let i = ((y * image.width + x) * 4) as usize;
        &image.rgba[i..i + 4]
    }

    fn close(actual: [f64; 2], expected: [f64; 2]) {
        for i in 0..2 {
            assert!(
                (actual[i] - expected[i]).abs() < 1e-9,
                "{actual:?} != {expected:?}"
            );
        }
    }

    #[test]
    fn overlay_transform_two_pairs_and_inverse_use_displayed_clockwise_coordinates() {
        let manual = OverlayTransform {
            translation: [40.0, -12.0],
            rotation_degrees: 90.0,
            scale: 2.0,
        };
        close(manual.apply([3.0, 4.0]), [32.0, -6.0]);
        close(manual.inverse([32.0, -6.0]), [3.0, 4.0]);
        let revision = [[10.0, 20.0], [30.0, 20.0]];
        let base = [[100.0, 100.0], [100.0, 140.0]];
        let transform = OverlayTransform::from_anchors(base, revision).unwrap();
        assert_eq!(transform.scale, 2.0);
        assert_eq!(transform.rotation_degrees, 90.0);
        for i in 0..2 {
            close(transform.apply(revision[i]), base[i]);
        }
        close(transform.inverse([96.0, 106.0]), [13.0, 22.0]);
        close(OverlayTransform::default().apply([19.0, 7.0]), [19.0, 7.0]);
    }

    #[test]
    fn overlay_parameters_reject_nonfinite_degenerate_and_unbounded_values() {
        for scale in [0.0, -1.0, 0.009, 100.01, f64::NAN, f64::INFINITY] {
            assert!(OverlayTransform {
                scale,
                ..Default::default()
            }
            .validate()
            .is_err());
        }
        for rotation_degrees in [361.0, -361.0, f64::NAN] {
            assert!(OverlayTransform {
                rotation_degrees,
                ..Default::default()
            }
            .validate()
            .is_err());
        }
        for value in [1_000_001.0, -1_000_001.0, f64::NAN] {
            assert!(OverlayTransform {
                translation: [value, 0.0],
                ..Default::default()
            }
            .validate()
            .is_err());
        }
        for points in [
            [[0.0, 0.0], [0.0, 0.0]],
            [[0.0, 0.0], [0.0001, 0.0]],
            [[-1.0, 0.0], [10.0, 10.0]],
            [[f64::NAN, 0.0], [10.0, 10.0]],
        ] {
            assert!(OverlayTransform::from_anchors(points, [[0.0, 0.0], [10.0, 0.0]]).is_err());
            assert!(OverlayTransform::from_anchors([[0.0, 0.0], [10.0, 0.0]], points).is_err());
        }
        for opacity in [-0.01, 1.01, f32::NAN] {
            assert!(OverlayOptions {
                opacity,
                ..Default::default()
            }
            .validate()
            .is_err());
        }
        for long_side in [0, 63, 2049, u32::MAX] {
            assert!(OverlayOptions {
                long_side,
                ..Default::default()
            }
            .validate()
            .is_err());
        }
        assert!(OverlayOptions {
            base_page: 0,
            ..Default::default()
        }
        .validate()
        .is_err());
        assert!(OverlayOptions {
            revision_page: 0,
            ..Default::default()
        }
        .validate()
        .is_err());
        assert!(OverlayOptions::default().validate().is_ok());
    }

    #[test]
    fn overlay_raster_translation_rotation_scale_clipping_and_opacity_are_explicit() {
        let cancel = AtomicBool::new(false);
        let white = raster(4, 4, &[]);
        let black = raster(4, 4, &[(0, 0)]);
        let run = |transform, opacity| {
            composite(
                &white, &black, [4.0; 2], [4.0; 2], transform, opacity, &cancel,
            )
            .unwrap()
        };
        let identity = run(OverlayTransform::default(), 1.0);
        assert_eq!(pixel(&identity, 0, 0), [0, 255, 255, 255]);
        assert_eq!(pixel(&identity, 1, 0), [255; 4]);
        let translation = run(
            OverlayTransform {
                translation: [2.0, 1.0],
                ..Default::default()
            },
            1.0,
        );
        assert_eq!(pixel(&translation, 2, 1), [0, 255, 255, 255]);
        assert_eq!(pixel(&translation, 0, 0), [255; 4]);
        let rotation = run(
            OverlayTransform {
                translation: [4.0, 0.0],
                rotation_degrees: 90.0,
                scale: 1.0,
            },
            1.0,
        );
        assert_eq!(pixel(&rotation, 3, 0), [0, 255, 255, 255]);
        let scale = run(
            OverlayTransform {
                scale: 2.0,
                ..Default::default()
            },
            1.0,
        );
        assert_eq!(pixel(&scale, 0, 0), [0, 255, 255, 255]);
        assert!(pixel(&scale, 1, 1)[0] < 255);
        let clipped = run(
            OverlayTransform {
                translation: [-4.0, -4.0],
                ..Default::default()
            },
            1.0,
        );
        assert_eq!(clipped.rgba, white.rgba);
        assert_eq!(run(OverlayTransform::default(), 0.0).rgba, white.rgba);
        let both = composite(
            &black,
            &black,
            [4.0; 2],
            [4.0; 2],
            OverlayTransform::default(),
            1.0,
            &cancel,
        )
        .unwrap();
        assert_eq!(pixel(&both, 0, 0), [0, 0, 255, 255]);
        assert_eq!(both.width, white.width);
        assert_eq!(both.height, white.height);
        cancel.store(true, Ordering::Relaxed);
        assert!(matches!(
            composite(
                &white,
                &black,
                [4.0; 2],
                [4.0; 2],
                OverlayTransform::default(),
                1.0,
                &cancel
            ),
            Err(EngineError::Cancelled)
        ));
    }

    #[test]
    fn overlay_sampling_preserves_physical_scale_and_composites_alpha_on_white() {
        let base = raster(4, 4, &[]);
        let mut revision = raster(8, 4, &[(2, 1)]);
        let cancel = AtomicBool::new(false);
        let output = composite(
            &base,
            &revision,
            [4.0; 2],
            [8.0, 4.0],
            OverlayTransform::default(),
            1.0,
            &cancel,
        )
        .unwrap();
        assert_eq!(pixel(&output, 2, 1), [0, 255, 255, 255]);
        assert_eq!(pixel(&output, 1, 1), [255; 4]);
        revision.rgba[((8 + 2) * 4 + 3) as usize] = 0;
        let transparent = composite(
            &base,
            &revision,
            [4.0; 2],
            [8.0, 4.0],
            OverlayTransform::default(),
            1.0,
            &cancel,
        )
        .unwrap();
        assert_eq!(transparent.rgba, base.rgba);
    }
}
