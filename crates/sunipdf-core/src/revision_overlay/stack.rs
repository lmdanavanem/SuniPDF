//! Independently styled raster layers, aligned to one fixed base-page canvas.
//!
//! Sources are opened and released one at a time. The retained source rasters and composite
//! together occupy at most 80 MiB; source PDF reads retain the separate 64 MiB per-file limit.

use super::{
    check_cancel, invalid, luminance, render_source, sample, OverlayAlignment, OverlayTransform,
    Source, DEFAULT_LONG_SIDE, MAX_LONG_SIDE,
};
use crate::local_review::{check_permission, digest, read_pdf};
use crate::{Engine, EngineError, RenderedPage};
use image::ImageEncoder;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub const MIN_LAYERS: usize = 2;
pub const MAX_LAYERS: usize = 4;
pub const MAX_RASTER_BYTES: usize = 80 * 1024 * 1024;

/// A page and its independent style. Coordinates use displayed points, top-left origin.
/// The first layer defines the canvas and must have an identity manual alignment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayStackLayer {
    pub path: PathBuf,
    /// Physical one-based page number.
    pub page: usize,
    pub alignment: OverlayAlignment,
    pub color: [u8; 3],
    pub opacity: f32,
    pub visible: bool,
}

impl OverlayStackLayer {
    /// The first two defaults reproduce the original magenta/cyan overlay.
    pub fn new(path: PathBuf, page: usize, index: usize) -> Self {
        const COLORS: [[u8; 3]; MAX_LAYERS] =
            [[255, 0, 255], [0, 255, 255], [0, 180, 70], [255, 140, 0]];
        Self {
            path,
            page,
            alignment: OverlayAlignment::default(),
            color: COLORS[index.min(MAX_LAYERS - 1)],
            opacity: if index == 0 { 1.0 } else { 0.65 },
            visible: true,
        }
    }

    fn resolved_transform(&self) -> Result<OverlayTransform, EngineError> {
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayStackOptions {
    pub layers: Vec<OverlayStackLayer>,
    pub long_side: u32,
}

impl Default for OverlayStackOptions {
    fn default() -> Self {
        Self {
            layers: Vec::new(),
            long_side: DEFAULT_LONG_SIDE,
        }
    }
}

impl OverlayStackOptions {
    pub fn validate(&self) -> Result<(), EngineError> {
        if !(MIN_LAYERS..=MAX_LAYERS).contains(&self.layers.len()) {
            return Err(invalid("Choose two to four overlay layers"));
        }
        if !(64..=MAX_LONG_SIDE).contains(&self.long_side) {
            return Err(invalid("Overlay requires a raster side from 64 to 2048"));
        }
        for (index, layer) in self.layers.iter().enumerate() {
            if layer.path.as_os_str().is_empty() || layer.page == 0 {
                return Err(invalid(
                    "Choose a PDF and a page number starting at 1 for every layer",
                ));
            }
            if !layer.opacity.is_finite() || !(0.0..=1.0).contains(&layer.opacity) {
                return Err(invalid("Layer opacity must be finite and between 0 and 1"));
            }
            layer.resolved_transform()?;
            if index == 0 && layer.alignment != OverlayAlignment::default() {
                return Err(invalid(
                    "The first overlay layer must use identity manual alignment",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
struct PinnedSource {
    selected_path: PathBuf,
    source: Source,
}

fn check_selected_path(selected: &Path, canonical: &Path) -> Result<(), EngineError> {
    if selected.canonicalize()? != canonical || canonical.canonicalize()? != canonical {
        return Err(invalid(
            "An overlay source path changed; prepare the overlay again",
        ));
    }
    Ok(())
}

fn checked_raster_bytes(
    images: &[RenderedPage],
    canvas_bytes: usize,
) -> Result<usize, EngineError> {
    images.iter().try_fold(canvas_bytes, |total, image| {
        checked_memory_add(total, image.rgba.len())
    })
}

fn checked_memory_add(total: usize, additional: usize) -> Result<usize, EngineError> {
    total
        .checked_add(additional)
        .filter(|bytes| *bytes <= MAX_RASTER_BYTES)
        .ok_or(EngineError::RenderTooLarge)
}

/// A complete immutable review snapshot. Passwords and source PDF bytes are not retained.
pub struct OverlayStackPreview {
    options: OverlayStackOptions,
    sources: Arc<[PinnedSource]>,
    transforms: Vec<OverlayTransform>,
    images: Arc<[RenderedPage]>,
    image: RenderedPage,
}

impl std::fmt::Debug for OverlayStackPreview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OverlayStackPreview")
            .field("options", &self.options)
            .field("sources", &self.sources)
            .field("transforms", &self.transforms)
            .finish_non_exhaustive()
    }
}

impl OverlayStackPreview {
    /// Recompose retained rasters without opening or rasterizing the source PDFs again.
    /// Every source is still checked, including hidden ones, before publishing the result.
    pub fn with_visibility(
        &self,
        visible: &[bool],
        cancel: &AtomicBool,
    ) -> Result<Self, EngineError> {
        if visible.len() != self.options.layers.len() {
            return Err(invalid("Provide visibility for every overlay layer"));
        }
        self.revalidate_sources(cancel)?;
        let mut options = self.options.clone();
        for (layer, &visible) in options.layers.iter_mut().zip(visible) {
            layer.visible = visible;
        }
        options.validate()?;
        let image = composite_stack(
            &self.images,
            &self.sources,
            &options.layers,
            &self.transforms,
            cancel,
        )?;
        self.revalidate_sources(cancel)?;
        Ok(Self {
            options,
            sources: self.sources.clone(),
            transforms: self.transforms.clone(),
            images: self.images.clone(),
            image,
        })
    }

    pub fn image(&self) -> &RenderedPage {
        &self.image
    }

    pub fn layer_image(&self, index: usize) -> Option<&RenderedPage> {
        self.images.get(index)
    }

    pub fn options(&self) -> &OverlayStackOptions {
        &self.options
    }

    /// Identity of a source retained by this preview, including hidden layers.
    pub fn source_sha256(&self, index: usize) -> Option<&str> {
        self.sources
            .get(index)
            .map(|pinned| pinned.source.source_sha256.as_str())
    }

    pub(crate) fn canonical_source_path(&self, index: usize) -> Option<&Path> {
        self.sources
            .get(index)
            .map(|pinned| pinned.source.path.as_path())
    }

    pub fn transform(&self, index: usize) -> Option<OverlayTransform> {
        self.transforms.get(index).copied()
    }

    /// Metadata describes the retained snapshot; export methods also revalidate every source.
    pub fn report(&self) -> Value {
        json!({
            "schema": "sunipdf-revision-overlay-stack", "version": 1,
            "coordinates": "display_top_left_points",
            "transform_direction": "layer_to_base", "rotation_direction": "clockwise",
            "layers": self.sources.iter().enumerate().map(|(index, pinned)| {
                let layer = &self.options.layers[index];
                json!({
                    "index": index, "source": pinned.source, "selected_path": pinned.selected_path,
                    "color": layer.color, "opacity": layer.opacity, "visible": layer.visible,
                    "alignment": layer.alignment, "transform": self.transforms[index],
                })
            }).collect::<Vec<_>>(),
            "canvas": {"width": self.image.width, "height": self.image.height, "base_layer": 0, "clip_to_base": true},
            "long_side": self.options.long_side, "sampling": "bilinear",
            "composition": "multiply_luminance_tints_on_white",
            "retained_raster_bytes": self.images.iter().map(|image| image.rgba.len()).sum::<usize>() + self.image.rgba.len(),
            "limitations": ["raster_preview", "base_page_clipping", "independently_bounded_source_rasters", "no_automatic_alignment"],
        })
    }

    pub fn revalidate_sources(&self, cancel: &AtomicBool) -> Result<(), EngineError> {
        for pinned in self.sources.iter() {
            check_cancel(cancel)?;
            check_selected_path(&pinned.selected_path, &pinned.source.path)?;
            if digest(&read_pdf(&pinned.source.path)?) != pinned.source.source_sha256 {
                return Err(invalid(
                    "An overlay source changed; prepare the overlay again",
                ));
            }
            check_selected_path(&pinned.selected_path, &pinned.source.path)?;
        }
        check_cancel(cancel)
    }

    /// Return report data; the caller owns atomic creation of a new output file.
    pub fn export_json(&self, cancel: &AtomicBool) -> Result<Value, EngineError> {
        self.revalidate_sources(cancel)?;
        Ok(self.report())
    }

    /// Return encoded data; the caller owns atomic creation of a new output file.
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

/// Permission-check and render every source, including hidden layers, one native document at a time.
/// Password entries correspond to layer positions and are never retained by the returned preview.
pub fn prepare_overlay_stack(
    engine: &Engine,
    mut options: OverlayStackOptions,
    passwords: &[Option<&str>],
    cancel: &AtomicBool,
) -> Result<OverlayStackPreview, EngineError> {
    options.validate()?;
    if passwords.len() != options.layers.len() {
        return Err(invalid(
            "Provide one optional password entry for each overlay layer",
        ));
    }
    check_cancel(cancel)?;
    let mut sources: Vec<PinnedSource> = Vec::with_capacity(options.layers.len());
    let mut images = Vec::with_capacity(options.layers.len());
    let mut transforms = Vec::with_capacity(options.layers.len());
    for (layer, password) in options.layers.iter_mut().zip(passwords.iter()) {
        check_cancel(cancel)?;
        let selected_path = std::path::absolute(&layer.path)?;
        let path = selected_path.canonicalize()?;
        check_selected_path(&selected_path, &path)?;
        let bytes = read_pdf(&path)?;
        let hash = digest(&bytes);
        check_cancel(cancel)?;
        let doc = engine.open_bytes(bytes, *password, &path)?;
        check_permission(&doc)?;
        check_cancel(cancel)?;
        let (source, image) = render_source(&doc, path, hash, layer.page, options.long_side)?;
        drop(doc);
        check_cancel(cancel)?;
        check_selected_path(&selected_path, &source.path)?;
        if let OverlayAlignment::TwoPoints { base, revision } = layer.alignment {
            sources[0].source.validate_anchors(base)?;
            source.validate_anchors(revision)?;
        }
        transforms.push(layer.resolved_transform()?);
        layer.path = selected_path.clone();
        sources.push(PinnedSource {
            selected_path,
            source,
        });
        images.push(image);
        checked_raster_bytes(&images, images[0].rgba.len())?;
    }
    let image = composite_stack(&images, &sources, &options.layers, &transforms, cancel)?;
    let preview = OverlayStackPreview {
        options,
        sources: sources.into(),
        transforms,
        images: images.into(),
        image,
    };
    preview.revalidate_sources(cancel)?;
    Ok(preview)
}

fn composite_stack(
    images: &[RenderedPage],
    sources: &[PinnedSource],
    layers: &[OverlayStackLayer],
    transforms: &[OverlayTransform],
    cancel: &AtomicBool,
) -> Result<RenderedPage, EngineError> {
    checked_raster_bytes(images, images[0].rgba.len())?;
    let base = &images[0];
    let base_size = [
        sources[0].source.display_width_pt,
        sources[0].source.display_height_pt,
    ];
    let inverse: Vec<_> = transforms
        .iter()
        .map(OverlayTransform::inverse_linear)
        .collect();
    let step_x = base_size[0] / f64::from(base.width);
    let step_y = base_size[1] / f64::from(base.height);
    let mut rgba = Vec::with_capacity(base.rgba.len());
    for y in 0..base.height {
        if y.is_multiple_of(32) {
            check_cancel(cancel)?;
        }
        let py = (f64::from(y) + 0.5) * step_y;
        for x in 0..base.width {
            let px = (f64::from(x) + 0.5) * step_x;
            let mut rgb = [1.0; 3];
            for (index, layer) in layers.iter().enumerate() {
                if !layer.visible || layer.opacity == 0.0 {
                    continue;
                }
                let gray = if index == 0 {
                    luminance(base, x, y)
                } else {
                    let [cos, sin] = inverse[index];
                    let dx = px - transforms[index].translation[0];
                    let dy = py - transforms[index].translation[1];
                    let point = [cos * dx + sin * dy, -sin * dx + cos * dy];
                    let source = &sources[index].source;
                    sample(
                        &images[index],
                        [source.display_width_pt, source.display_height_pt],
                        point,
                    )
                };
                let density = (1.0 - gray) * f64::from(layer.opacity);
                for (channel, color) in rgb.iter_mut().zip(layer.color) {
                    let tint = if color == 255 {
                        1.0
                    } else if index == 0 && color == 0 && layer.opacity == 1.0 {
                        gray
                    } else {
                        1.0 - density * (1.0 - f64::from(color) / 255.0)
                    };
                    *channel *= tint;
                }
            }
            rgba.extend(rgb.map(|channel| (channel * 255.0).round() as u8));
            rgba.push(255);
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
    use std::sync::atomic::Ordering;

    fn raster(pixels: &[[u8; 4]]) -> RenderedPage {
        RenderedPage {
            page_index: 0,
            width: pixels.len() as u32,
            height: 1,
            rgba: pixels.iter().flatten().copied().collect(),
        }
    }

    fn source(index: usize, image: &RenderedPage) -> PinnedSource {
        PinnedSource {
            selected_path: format!("layer-{index}.pdf").into(),
            source: Source {
                path: format!("layer-{index}.pdf").into(),
                source_sha256: String::new(),
                page: 1,
                page_count: 1,
                display_width_pt: f64::from(image.width),
                display_height_pt: f64::from(image.height),
                crop_box_user_pt: [0.0, 0.0, image.width as f32, image.height as f32],
                rotation_degrees: 0,
                raster_width: image.width,
                raster_height: image.height,
            },
        }
    }

    fn layers(count: usize) -> Vec<OverlayStackLayer> {
        (0..count)
            .map(|index| OverlayStackLayer::new(format!("layer-{index}.pdf").into(), 1, index))
            .collect()
    }

    #[test]
    fn stack_defaults_match_legacy_pixels_for_alpha_gray_and_transforms() {
        let cancel = AtomicBool::new(false);
        let images = vec![
            raster(&[
                [0, 0, 0, 255],
                [40, 100, 190, 255],
                [37, 84, 211, 70],
                [255; 4],
            ]),
            raster(&[
                [255; 4],
                [128, 128, 128, 255],
                [0, 0, 0, 255],
                [23, 88, 180, 120],
            ]),
        ];
        let sources: Vec<_> = images
            .iter()
            .enumerate()
            .map(|(index, image)| source(index, image))
            .collect();
        for opacity in [0.0, 0.25, 0.65, 1.0] {
            for transform in [
                OverlayTransform::default(),
                OverlayTransform {
                    translation: [0.3, -0.2],
                    rotation_degrees: 17.5,
                    scale: 0.75,
                },
            ] {
                let mut layers = layers(2);
                layers[1].opacity = opacity;
                let stack = composite_stack(
                    &images,
                    &sources,
                    &layers,
                    &[OverlayTransform::default(), transform],
                    &cancel,
                )
                .unwrap();
                let legacy = super::super::composite(
                    &images[0],
                    &images[1],
                    [4.0, 1.0],
                    [4.0, 1.0],
                    transform,
                    opacity,
                    &cancel,
                )
                .unwrap();
                assert_eq!(
                    stack.rgba, legacy.rgba,
                    "opacity={opacity}, transform={transform:?}"
                );
            }
        }
    }

    #[test]
    fn stack_layer_tints_visibility_opacity_and_fixed_canvas_are_independent() {
        let cancel = AtomicBool::new(false);
        let images = vec![
            raster(&[[0, 0, 0, 255], [255; 4]]),
            raster(&[[0, 0, 0, 255], [255; 4]]),
            raster(&[[0, 0, 0, 255], [255; 4]]),
        ];
        let sources: Vec<_> = images
            .iter()
            .enumerate()
            .map(|(index, image)| source(index, image))
            .collect();
        let transforms = vec![OverlayTransform::default(); 3];
        let mut layers = layers(3);
        layers[1].opacity = 1.0;
        layers[2].color = [255, 255, 0];
        layers[2].opacity = 0.5;
        let render = |layers: &[OverlayStackLayer]| {
            composite_stack(&images, &sources, layers, &transforms, &cancel).unwrap()
        };
        assert_eq!(render(&layers).rgba, [0, 0, 128, 255, 255, 255, 255, 255]);
        layers[0].visible = false;
        assert_eq!(render(&layers).rgba, [0, 255, 128, 255, 255, 255, 255, 255]);
        layers[1].visible = false;
        assert_eq!(
            render(&layers).rgba,
            [255, 255, 128, 255, 255, 255, 255, 255]
        );
        layers[2].opacity = 0.0;
        assert_eq!(render(&layers).rgba, [255; 8]);
        assert_eq!((render(&layers).width, render(&layers).height), (2, 1));
        cancel.store(true, Ordering::Relaxed);
        assert!(matches!(
            composite_stack(&images, &sources, &layers, &transforms, &cancel),
            Err(EngineError::Cancelled)
        ));
    }

    #[test]
    fn stack_validates_all_layers_bounds_and_checked_memory_budget() {
        let valid = OverlayStackOptions {
            layers: layers(4),
            long_side: 2048,
        };
        assert!(valid.validate().is_ok());
        for count in [0, 1, 5] {
            assert!(OverlayStackOptions {
                layers: layers(count),
                ..valid.clone()
            }
            .validate()
            .is_err());
        }
        for long_side in [0, 63, 2049, u32::MAX] {
            assert!(OverlayStackOptions {
                long_side,
                ..valid.clone()
            }
            .validate()
            .is_err());
        }
        for opacity in [-0.01, 1.01, f32::NAN, f32::INFINITY] {
            let mut bad = valid.clone();
            bad.layers[3].visible = false;
            bad.layers[3].opacity = opacity;
            assert!(bad.validate().is_err());
        }
        let mut bad = valid.clone();
        bad.layers
            .iter_mut()
            .for_each(|layer| layer.visible = false);
        assert!(bad.validate().is_ok());
        bad = valid.clone();
        bad.layers[0].alignment = OverlayAlignment::Manual {
            transform: OverlayTransform {
                translation: [1.0, 0.0],
                ..Default::default()
            },
        };
        assert!(bad.validate().is_err());
        bad = valid.clone();
        bad.layers[0].alignment = OverlayAlignment::TwoPoints {
            base: [[0.0, 0.0], [1.0, 1.0]],
            revision: [[0.0, 0.0], [1.0, 1.0]],
        };
        assert!(bad.validate().is_err());
        bad = valid.clone();
        bad.layers[3].page = 0;
        assert!(bad.validate().is_err());
        bad = valid.clone();
        bad.layers[3].path = PathBuf::new();
        assert!(bad.validate().is_err());
        bad = valid.clone();
        bad.layers[3].alignment = OverlayAlignment::Manual {
            transform: OverlayTransform {
                scale: f64::NAN,
                ..Default::default()
            },
        };
        assert!(bad.validate().is_err());
        let one_raster = 2048 * 2048 * 4;
        assert_eq!(
            checked_memory_add(4 * one_raster, one_raster).unwrap(),
            MAX_RASTER_BYTES
        );
        assert!(checked_memory_add(MAX_RASTER_BYTES, 1).is_err());
        assert!(checked_memory_add(usize::MAX, 1).is_err());
    }
}
