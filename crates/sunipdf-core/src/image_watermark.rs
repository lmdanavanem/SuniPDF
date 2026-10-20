//! Bounded, atomic image watermarks in displayed-page coordinates.

use crate::{Document, EngineError, StampPosition};
use image::DynamicImage;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct ImageWatermark {
    pub width_percent: f32,
    pub opacity: f32,
    pub angle: f32,
    pub position: StampPosition,
    pub margin_pt: f32,
}

impl Default for ImageWatermark {
    fn default() -> Self {
        Self {
            width_percent: 30.0,
            opacity: 0.3,
            angle: 0.0,
            position: StampPosition::Center,
            margin_pt: 28.0,
        }
    }
}

impl ImageWatermark {
    fn fitted_width(&self, page: (f32, f32), aspect: f32) -> Result<f32, EngineError> {
        if !self.width_percent.is_finite()
            || !(1.0..=100.0).contains(&self.width_percent)
            || !self.opacity.is_finite()
            || !(0.01..=1.0).contains(&self.opacity)
            || !self.angle.is_finite()
            || !self.margin_pt.is_finite()
            || self.margin_pt < 0.0
            || !aspect.is_finite()
            || aspect <= 0.0
        {
            return Err(EngineError::Other(
                "invalid image watermark settings".into(),
            ));
        }
        let (width, height) = (page.0 - 2.0 * self.margin_pt, page.1 - 2.0 * self.margin_pt);
        if width <= 0.0 || height <= 0.0 {
            return Err(EngineError::Other(
                "watermark margins leave no page area".into(),
            ));
        }
        let (sin, cos) = self.angle.rem_euclid(360.0).to_radians().sin_cos();
        let requested = page.0 * self.width_percent / 100.0;
        let fitted = requested
            .min(width / (cos.abs() + aspect * sin.abs()))
            .min(height / (sin.abs() + aspect * cos.abs()));
        if !fitted.is_finite() || fitted < 0.1 {
            return Err(EngineError::Other("watermark is too small".into()));
        }
        Ok(fitted)
    }
}

impl Document<'_> {
    /// Adds one editable foreground image per selected page; a refusal leaves the document intact.
    pub fn image_watermark(
        &mut self,
        pages: Option<&[usize]>,
        path: &Path,
        style: &ImageWatermark,
    ) -> Result<usize, EngineError> {
        if !self.properties().can_modify {
            return Err(EngineError::SecurityRestricted);
        }
        let mut targets = pages.map_or_else(
            || (0..self.page_count()).collect::<Vec<_>>(),
            <[usize]>::to_vec,
        );
        targets.sort_unstable();
        targets.dedup();
        if targets.is_empty() || targets.len() > 1000 {
            return Err(EngineError::Other(
                "select between 1 and 1000 watermark pages".into(),
            ));
        }
        // Validate settings and every destination before decoding or changing any page.
        for &page in &targets {
            let size = self.page_size(page)?;
            style.fitted_width((size.width_pt, size.height_pt), 1.0)?;
        }
        let metadata = std::fs::metadata(path)?;
        if !metadata.is_file() || metadata.len() > 64 * 1024 * 1024 {
            return Err(EngineError::Other(
                "watermark image must be a regular file of at most 64 MiB".into(),
            ));
        }
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(4096);
        limits.max_image_height = Some(4096);
        limits.max_alloc = Some(32 * 1024 * 1024);
        let mut image = crate::convert::decode(path, limits)?.into_rgba8();
        let pixels = u64::from(image.width()) * u64::from(image.height());
        if pixels == 0 || pixels > 4_000_000 || pixels * targets.len() as u64 > 64_000_000 {
            return Err(EngineError::Other(
                "watermark exceeds the 4-million-pixel image or 64-million-pixel page budget"
                    .into(),
            ));
        }
        let aspect = image.height() as f32 / image.width() as f32;
        let widths = targets
            .iter()
            .map(|&p| {
                let size = self.page_size(p)?;
                style.fitted_width((size.width_pt, size.height_pt), aspect)
            })
            .collect::<Result<Vec<_>, EngineError>>()?;
        let mut visible = false;
        for pixel in image.pixels_mut() {
            pixel[3] = (f32::from(pixel[3]) * style.opacity).round() as u8;
            visible |= pixel[3] > 0;
            if pixel[3] == 0 {
                pixel.0[..3].fill(0);
            }
        }
        if !visible {
            return Err(EngineError::Other(
                "watermark image is fully transparent".into(),
            ));
        }
        let image = DynamicImage::ImageRgba8(image);
        let mut scratch = Document {
            inner: self
                .pdfium
                .load_pdf_from_byte_vec(self.snapshot()?, self.password())?,
            pdfium: self.pdfium,
            info: self.info.clone(),
            page_rotations: self.page_rotations.clone(),
            password: self.password.clone(),
            font_cache: Default::default(),
        };
        for (&page, &width) in targets.iter().zip(&widths) {
            let size = scratch.page_size(page)?;
            scratch.add_image_data(page, &image, 0.0, 0.0, width)?;
            let index = scratch.page_objects(page)?.len() - 1;
            if style.angle.rem_euclid(360.0) != 0.0 {
                scratch.rotate_object(page, index, -style.angle.rem_euclid(360.0))?;
            }
            let b = scratch.object_info(page, index)?.bounds;
            let (w, h) = (b.right - b.left, b.top - b.bottom);
            let m = style.margin_pt;
            let (x, y) = match style.position {
                StampPosition::Center => ((size.width_pt - w) / 2.0, (size.height_pt - h) / 2.0),
                StampPosition::TopLeft => (m, size.height_pt - m - h),
                StampPosition::TopCenter => ((size.width_pt - w) / 2.0, size.height_pt - m - h),
                StampPosition::TopRight => (size.width_pt - m - w, size.height_pt - m - h),
                StampPosition::BottomLeft => (m, m),
                StampPosition::BottomCenter => ((size.width_pt - w) / 2.0, m),
                StampPosition::BottomRight => (size.width_pt - m - w, m),
            };
            scratch.move_object(page, index, x - b.left, y - b.bottom)?;
        }
        self.inner = scratch.inner;
        self.font_cache.clear();
        Ok(targets.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rotated_watermark_fit_respects_both_page_axes_and_rejects_invalid_options() {
        for angle in [0.0, 45.0, 90.0, -45.0, 370.0] {
            let style = ImageWatermark {
                width_percent: 100.0,
                angle,
                ..Default::default()
            };
            let width = style.fitted_width((200.0, 100.0), 2.0).unwrap();
            let (sin, cos) = angle.to_radians().sin_cos();
            assert!(width * (cos.abs() + 2.0 * sin.abs()) <= 144.01);
            assert!(width * (sin.abs() + 2.0 * cos.abs()) <= 44.01);
        }
        for value in [f32::NAN, f32::INFINITY, -1.0, 0.0, 101.0] {
            assert!(ImageWatermark {
                width_percent: value,
                ..Default::default()
            }
            .fitted_width((612.0, 792.0), 1.0)
            .is_err());
        }
        for value in [f32::NAN, -0.1, 0.0, 1.1] {
            assert!(ImageWatermark {
                opacity: value,
                ..Default::default()
            }
            .fitted_width((612.0, 792.0), 1.0)
            .is_err());
        }
        assert!(ImageWatermark::default()
            .fitted_width((20.0, 20.0), 1.0)
            .is_err());
    }
}
