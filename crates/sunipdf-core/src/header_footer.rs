//! Six bounded, non-overlapping text zones in displayed-page coordinates.

use crate::{Document, EngineError, TextStyle};

#[derive(Debug, Clone, PartialEq)]
pub struct HeaderFooter {
    /// Header left/center/right, then footer left/center/right. Empty zones are omitted.
    pub zones: [String; 6],
    pub style: TextStyle,
    pub margin_pt: f32,
}

impl Default for HeaderFooter {
    fn default() -> Self {
        Self {
            zones: Default::default(),
            style: TextStyle {
                size_pt: 10.0,
                ..Default::default()
            },
            margin_pt: 28.0,
        }
    }
}

impl HeaderFooter {
    pub fn validate(&self) -> Result<(), EngineError> {
        if self.zones.iter().all(|s| s.trim().is_empty())
            || self
                .zones
                .iter()
                .any(|s| s.chars().count() > 256 || s.chars().any(char::is_control))
            || !self.style.size_pt.is_finite()
            || !(4.0..=72.0).contains(&self.style.size_pt)
            || !self.style.opacity.is_finite()
            || !(0.05..=1.0).contains(&self.style.opacity)
            || !self.margin_pt.is_finite()
            || !(0.0..=1000.0).contains(&self.margin_pt)
        {
            return Err(EngineError::Other(
                "invalid header/footer text or settings".into(),
            ));
        }
        Ok(())
    }
}

impl Document<'_> {
    /// Adds at most six editable text objects per page. Refuses overflowing zones atomically.
    pub fn headers_footers(
        &mut self,
        pages: Option<&[usize]>,
        options: &HeaderFooter,
    ) -> Result<usize, EngineError> {
        options.validate()?;
        if !self.properties().can_modify {
            return Err(EngineError::SecurityRestricted);
        }
        let total = self.page_count();
        let mut targets = pages.map_or_else(|| (0..total).collect::<Vec<_>>(), <[usize]>::to_vec);
        targets.sort_unstable();
        targets.dedup();
        if targets.is_empty() || targets.len() > 1000 {
            return Err(EngineError::Other(
                "select between 1 and 1000 header/footer pages".into(),
            ));
        }
        for &page in &targets {
            self.page_size(page)?;
        }
        let mut scratch = Document {
            inner: self
                .pdfium
                .load_pdf_from_byte_vec(self.snapshot()?, self.password())?,
            pdfium: self.pdfium,
            info: self.info.clone(),
            page_rotations: self.page_rotations.clone(),
            password: self.password.clone(),
            font_cache: Default::default(),
            form_render_need_appearances: Default::default(),
        };
        for &page in &targets {
            let size = scratch.page_size(page)?;
            let margin = options.margin_pt;
            let available_width = size.width_pt - 2.0 * margin;
            let column = (available_width - 16.0) / 3.0;
            let row_height = (size.height_pt - 2.0 * margin - 8.0) / 2.0;
            if column < 1.0 || row_height < 1.0 {
                return Err(EngineError::Other(
                    "header/footer margins leave no page area".into(),
                ));
            }
            for (zone, template) in options
                .zones
                .iter()
                .enumerate()
                .filter(|(_, text)| !text.trim().is_empty())
            {
                let text = template
                    .replace("{n}", &(page + 1).to_string())
                    .replace("{total}", &total.to_string());
                scratch.add_text(page, 0.0, 0.0, &text, &options.style)?;
                let index = scratch.page_objects(page)?.len() - 1;
                let bounds = scratch.object_info(page, index)?.bounds;
                let width = bounds.right - bounds.left;
                let height = bounds.top - bounds.bottom;
                if !width.is_finite()
                    || !height.is_finite()
                    || width <= 0.0
                    || height <= 0.0
                    || width > column
                    || height > row_height
                {
                    return Err(EngineError::Other(format!(
                        "header/footer zone {} does not fit page {}",
                        zone + 1,
                        page + 1
                    )));
                }
                let x = match zone % 3 {
                    0 => margin,
                    1 => (size.width_pt - width) / 2.0,
                    _ => size.width_pt - margin - width,
                };
                let y = if zone < 3 {
                    size.height_pt - margin - height
                } else {
                    margin
                };
                scratch.move_object(page, index, x - bounds.left, y - bounds.bottom)?;
            }
        }
        self.form_render_need_appearances.set(None);
        self.inner = scratch.inner;
        self.font_cache.clear();
        Ok(targets.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn header_footer_validation_bounds_text_and_settings() {
        let mut options = HeaderFooter::default();
        assert!(options.validate().is_err());
        options.zones[0] = "Page {n} / {total}".into();
        assert!(options.validate().is_ok());
        for text in ["a\nb".to_string(), "a\tb".into(), "x".repeat(257)] {
            options.zones[0] = text;
            assert!(options.validate().is_err());
        }
        options.zones[0] = "valid".into();
        for value in [f32::NAN, f32::INFINITY, 0.0, 73.0] {
            options.style.size_pt = value;
            assert!(options.validate().is_err());
        }
    }
}
