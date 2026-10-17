//! Bounded plain-text paragraph reflow with the original PDF font and a native rendered preview.

use crate::edit::checked_indices;
use crate::{Document, EngineError, PixelRect, RenderedPage};
use pdfium_render::prelude::*;

pub struct ParagraphPreview {
    pub image: RenderedPage,
    pub scale: f32,
    pub region: PixelRect,
    pub objects: usize,
}

fn unsupported() -> EngineError {
    EngineError::Other(
        "paragraph editing requires compatible plain text without clipping or complex text state"
            .into(),
    )
}

fn preflight(bytes: &[u8], password: Option<&str>, page_index: usize) -> Result<(), EngineError> {
    use crate::security::lopdf_err;
    let pdf = crate::security::load(bytes, password)?;
    let catalog = pdf.catalog().map_err(lopdf_err)?;
    if catalog.has(b"StructTreeRoot") || catalog.has(b"OCProperties") {
        return Err(unsupported());
    }
    let page = *pdf
        .get_pages()
        .get(&(page_index as u32 + 1))
        .ok_or(EngineError::PageOutOfRange(page_index))?;
    let content = pdf
        .get_page_content_with_limit(page, 32 * 1024 * 1024)
        .map_err(lopdf_err)?;
    let content = lopdf::content::Content::decode_strict(&content).map_err(lopdf_err)?;
    for op in content.operations {
        if op.operator == "TJ" {
            let array = op
                .operands
                .first()
                .ok_or_else(unsupported)?
                .as_array()
                .map_err(lopdf_err)?;
            if array.iter().any(|value| {
                matches!(value, lopdf::Object::Integer(_) | lopdf::Object::Real(_))
                    && value.as_float().ok() != Some(0.0)
            }) {
                return Err(unsupported());
            }
        }
        let expected = match op.operator.as_str() {
            "Tc" | "Tw" | "Ts" | "Tr" => Some(0.0),
            "Tz" => Some(100.0),
            // A clip containing the old text can be optimized away by PDFium. A longer
            // replacement could leave it, so reject before any native normalization.
            "BMC" | "BDC" | "W" | "W*" => return Err(unsupported()),
            _ => None,
        };
        if let Some(expected) = expected {
            if op.operands.len() != 1 || op.operands[0].as_float().map_err(lopdf_err)? != expected {
                return Err(unsupported());
            }
        }
    }
    let (direct, inherited) = pdf.get_page_resources(page).map_err(lopdf_err)?;
    let mut resources = direct.into_iter().collect::<Vec<_>>();
    for id in inherited {
        resources.push(pdf.get_dictionary(id).map_err(lopdf_err)?);
    }
    for resources in resources {
        if let Ok(gs) = resources.get(b"ExtGState") {
            let (_, gs) = pdf.dereference(gs).map_err(lopdf_err)?;
            for (_, value) in gs.as_dict().map_err(lopdf_err)?.iter() {
                let (_, value) = pdf.dereference(value).map_err(lopdf_err)?;
                for (name, value) in value.as_dict().map_err(lopdf_err)?.iter() {
                    if name == b"Type" {
                        continue;
                    }
                    if name == b"ca" || name == b"CA" {
                        let alpha = value.as_float().map_err(lopdf_err)?;
                        if !alpha.is_finite() || !(0.0..=1.0).contains(&alpha) {
                            return Err(unsupported());
                        }
                        continue;
                    }
                    if name == b"BM" && value.as_name().ok() == Some(b"Normal".as_slice()) {
                        continue;
                    }
                    return Err(unsupported());
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn measure(page: &mut PdfPage<'_>, value: &str) -> Result<f32, EngineError> {
    let mut object = page.objects().get(0)?;
    object
        .as_text_object_mut()
        .ok_or_else(unsupported)?
        .set_text(value)?;
    drop(object);
    page.regenerate_content()?;
    let object = page.objects().get(0)?;
    let object = object.as_text_object().ok_or_else(unsupported)?;
    let text = page.text()?;
    let back = text.for_object(object);
    let squash = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    if squash(&back) != squash(value) {
        let mut seen = std::collections::HashSet::new();
        return Err(EngineError::MissingGlyphs(
            value
                .chars()
                .filter(|c| !c.is_whitespace() && !back.contains(*c) && seen.insert(*c))
                .collect(),
        ));
    }
    let chars = object.chars(&text)?;
    let mut right = 0.0f32;
    let mut left = 0.0f32;
    let mut baseline = None;
    for index in 0..chars.len() {
        let glyph = chars.get(index)?;
        if glyph.is_generated()? {
            continue;
        }
        let (x, y) = glyph.origin()?;
        let bounds = glyph.loose_bounds()?;
        if ![x.value, y.value, bounds.left().value, bounds.right().value]
            .iter()
            .all(|v| v.is_finite())
        {
            return Err(unsupported());
        }
        if baseline.is_some_and(|base: f32| (base - y.value).abs() > 0.01) {
            return Err(unsupported());
        }
        baseline = Some(y.value);
        left = left.min(bounds.left().value);
        right = right.max(bounds.right().value);
    }
    let width = right - left;
    if !width.is_finite() || width <= 0.0 {
        return Err(unsupported());
    }
    Ok(width)
}

impl<'a> Document<'a> {
    fn paragraph_candidate(
        &self,
        page_index: usize,
        indices: &[usize],
        text: &str,
        width: f32,
        leading: f32,
    ) -> Result<(Document<'a>, Vec<usize>), EngineError> {
        if text.trim().is_empty()
            || text.chars().count() > 10_000
            || text
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
            || !width.is_finite()
            || !(1.0..=10_000.0).contains(&width)
            || !leading.is_finite()
            || !(1.0..=3.0).contains(&leading)
        {
            return Err(EngineError::Other(
                "invalid paragraph or layout limits".into(),
            ));
        }
        let bytes = self.snapshot()?;
        preflight(&bytes, self.password(), page_index)?;
        let mut scratch = Document {
            inner: self.pdfium.load_pdf_from_byte_vec(bytes, self.password())?,
            pdfium: self.pdfium,
            info: self.info.clone(),
            page_rotations: self.page_rotations.clone(),
            password: self.password.clone(),
            font_cache: Default::default(),
        };
        scratch.merge_content_streams(page_index)?;
        let (indices, font, size, matrix, color) = {
            let page = scratch.page_handle(page_index)?;
            let indices = checked_indices(indices, page.objects().len())?;
            if indices.windows(2).any(|pair| pair[1] != pair[0] + 1) {
                return Err(unsupported());
            }
            let first = page.objects().get(indices[0] as PdfPageObjectIndex)?;
            let first_text = first.as_text_object().ok_or_else(unsupported)?;
            let font = first_text.font().token();
            let size = first_text.unscaled_font_size();
            let matrix = first_text.matrix()?;
            if ![
                matrix.a(),
                matrix.b(),
                matrix.c(),
                matrix.d(),
                matrix.e(),
                matrix.f(),
                size.value,
            ]
            .iter()
            .all(|value| value.is_finite())
                || size.value <= 0.0
            {
                return Err(unsupported());
            }
            let color = first.fill_color()?;
            for &index in &indices {
                let object = page.objects().get(index as PdfPageObjectIndex)?;
                let text = object.as_text_object().ok_or_else(unsupported)?;
                let m = text.matrix()?;
                let c = object.fill_color()?;
                if ![m.a(), m.b(), m.c(), m.d(), m.e(), m.f()]
                    .iter()
                    .all(|value| value.is_finite())
                    || object.get_clip_path().is_some_and(|clip| !clip.is_empty())
                    || text.render_mode() != PdfPageTextRenderMode::FilledUnstroked
                    || text.font().token() != font
                    || text.unscaled_font_size() != size
                    || [
                        m.a() - matrix.a(),
                        m.b() - matrix.b(),
                        m.c() - matrix.c(),
                        m.d() - matrix.d(),
                    ]
                    .iter()
                    .any(|v| v.abs() > 0.0001)
                    || (c.red(), c.green(), c.blue(), c.alpha())
                        != (color.red(), color.green(), color.blue(), color.alpha())
                {
                    return Err(unsupported());
                }
            }
            (indices, font, size, matrix, color)
        };
        let x_scale = matrix.a().hypot(matrix.b());
        if !x_scale.is_finite()
            || x_scale <= 0.0
            || !matrix.determinant().is_finite()
            || matrix.determinant() == 0.0
        {
            return Err(unsupported());
        }
        let probe_index = scratch.inner.pages().len();
        drop(
            scratch
                .inner
                .pages_mut()
                .create_page_at_end(PdfPagePaperSize::a4())?,
        );
        let mut probe = scratch.inner.pages().last()?;
        let object = PdfPageTextObject::new(&scratch.inner, " ", font, size)?;
        probe.objects_mut().add_text_object(object)?;
        let mut lines = Vec::new();
        for hard_line in text.lines() {
            let mut line = String::new();
            for word in hard_line.split_whitespace() {
                let candidate = if line.is_empty() {
                    word.to_string()
                } else {
                    format!("{line} {word}")
                };
                if measure(&mut probe, &candidate)? * x_scale <= width {
                    line = candidate;
                } else {
                    if line.is_empty() || measure(&mut probe, word)? * x_scale > width {
                        return Err(EngineError::Other(
                            "a paragraph word exceeds the chosen width".into(),
                        ));
                    }
                    lines.push(std::mem::take(&mut line));
                    line.push_str(word);
                }
                if lines.len() > 512 {
                    return Err(EngineError::Other("paragraph line limit exceeded".into()));
                }
            }
            lines.push(line);
            if lines.len() > 512 {
                return Err(EngineError::Other("paragraph line limit exceeded".into()));
            }
        }
        drop(probe);
        scratch.inner.pages().get(probe_index)?.delete()?;
        let mut replacements = Vec::new();
        for (line, value) in lines.iter().enumerate() {
            if value.is_empty() {
                continue;
            }
            let mut object = PdfPageTextObject::new(&scratch.inner, value, font, size)?;
            object.set_fill_color(color)?;
            let dy = -(line as f32) * size.value * leading;
            object.transform(
                matrix.a(),
                matrix.b(),
                matrix.c(),
                matrix.d(),
                matrix.e() + matrix.c() * dy,
                matrix.f() + matrix.d() * dy,
            )?;
            replacements.push(object);
        }
        let first = indices[0];
        let mut page = scratch.page_handle(page_index)?;
        let strategy = page.content_regeneration_strategy();
        page.set_content_regeneration_strategy(PdfPageContentRegenerationStrategy::Manual);
        for &index in indices.iter().rev() {
            page.objects_mut()
                .remove_object_at_index(index as PdfPageObjectIndex)?;
        }
        let mut new_indices = Vec::new();
        for (offset, object) in replacements.into_iter().enumerate() {
            let index = first + offset;
            page.objects_mut()
                .insert_object_at_index(index as PdfPageObjectIndex, PdfPageObject::Text(object))?;
            new_indices.push(index);
        }
        page.regenerate_content()?;
        page.set_content_regeneration_strategy(strategy);
        drop(page);
        let page_size = scratch.page_size(page_index)?;
        let mut back = String::new();
        for &index in &new_indices {
            let info = scratch.object_info(page_index, index)?;
            back.push_str(info.text.as_deref().ok_or_else(unsupported)?);
            let b = info.bounds;
            if b.left < 0.0
                || b.bottom < 0.0
                || b.right > page_size.width_pt
                || b.top > page_size.height_pt
            {
                return Err(EngineError::Other(
                    "paragraph extends outside the displayed page".into(),
                ));
            }
        }
        let squash = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
        if squash(&back) != squash(text) {
            return Err(EngineError::Other("paragraph text readback differs".into()));
        }
        Ok((scratch, new_indices))
    }

    pub fn reflow_paragraph(
        &mut self,
        page_index: usize,
        indices: &[usize],
        text: &str,
        width: f32,
        leading: f32,
    ) -> Result<Vec<usize>, EngineError> {
        let (scratch, indices) =
            self.paragraph_candidate(page_index, indices, text, width, leading)?;
        self.inner = scratch.inner;
        self.font_cache.clear();
        Ok(indices)
    }

    pub fn preview_paragraph(
        &self,
        page_index: usize,
        indices: &[usize],
        text: &str,
        width: f32,
        leading: f32,
    ) -> Result<ParagraphPreview, EngineError> {
        let (scratch, indices) =
            self.paragraph_candidate(page_index, indices, text, width, leading)?;
        let mut bounds = scratch.object_info(page_index, indices[0])?.bounds;
        for index in indices.iter().skip(1) {
            let b = scratch.object_info(page_index, *index)?.bounds;
            bounds.left = bounds.left.min(b.left);
            bounds.bottom = bounds.bottom.min(b.bottom);
            bounds.right = bounds.right.max(b.right);
            bounds.top = bounds.top.max(b.top);
        }
        let size = scratch.page_size(page_index)?;
        let scale = 2.0f32
            .min(1200.0 / (bounds.right - bounds.left + 8.0).max(bounds.top - bounds.bottom + 8.0));
        let (page_w, page_h) =
            crate::tiles::page_pixels(size, scale).ok_or(EngineError::RenderTooLarge)?;
        let x = ((bounds.left - 4.0).max(0.0) * scale).floor() as u32;
        let y = ((size.height_pt - bounds.top - 4.0).max(0.0) * scale).floor() as u32;
        let right = (((bounds.right + 4.0).min(size.width_pt) * scale).ceil() as u32).min(page_w);
        let bottom = (((size.height_pt - bounds.bottom + 4.0).min(size.height_pt) * scale).ceil()
            as u32)
            .min(page_h);
        let region = PixelRect::new(x, y, right - x, bottom - y);
        let image = scratch.render_region(page_index, scale, region)?;
        Ok(ParagraphPreview {
            image,
            scale,
            region,
            objects: indices.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_preflight_rejects_clips_and_complex_text_state_before_native_normalization() {
        for (prefix, suffix) in [
            ("1 Tw\n", ""),
            ("/Span BMC\n", "\nEMC"),
            ("q 0 0 612 792 re W n\n", "\nQ"),
            ("q 0 0 612 792 re W* n\n", "\nQ"),
        ] {
            let mut pdf =
                lopdf::Document::load_mem(include_bytes!("../tests/fixtures/two-pages.pdf"))
                    .unwrap();
            let page = pdf.get_pages()[&1];
            let mut content = prefix.as_bytes().to_vec();
            content.extend(pdf.get_page_content(page));
            content.extend(suffix.as_bytes());
            let stream = pdf.add_object(lopdf::Stream::new(lopdf::Dictionary::new(), content));
            pdf.get_dictionary_mut(page)
                .unwrap()
                .set("Contents", stream);
            let mut bytes = Vec::new();
            pdf.save_to(&mut bytes).unwrap();
            assert!(
                preflight(&bytes, None, 0).is_err(),
                "unsupported state {prefix:?}"
            );
        }
    }
}
