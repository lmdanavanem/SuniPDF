//! Conservative editable text/cell exports and explicit raster-slide export, always to a new copy.

use crate::{Document, EngineError, ObjectInfo, ObjectKind, PageSize};
use pdfium_render::prelude::PdfPageObjectsCommon;
use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

#[path = "export_office_zip.rs"]
pub(crate) mod archive;
#[path = "export_office_parts.rs"]
mod parts;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Format {
    #[default]
    Word,
    Excel,
    PowerPoint,
}
impl Format {
    pub fn id(self) -> &'static str {
        match self {
            Self::Word => "word",
            Self::Excel => "excel",
            Self::PowerPoint => "powerpoint",
        }
    }
    pub fn extension(self) -> &'static str {
        match self {
            Self::Word => "docx",
            Self::Excel => "xlsx",
            Self::PowerPoint => "pptx",
        }
    }
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "word" | "docx" => Some(Self::Word),
            "excel" | "xlsx" => Some(Self::Excel),
            "powerpoint" | "pptx" => Some(Self::PowerPoint),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExportReport {
    pub format: Format,
    pub pages: usize,
    pub editable_text_chars: usize,
    pub rows: usize,
    pub cells: usize,
    pub page_images: usize,
    pub bytes: usize,
}
impl ExportReport {
    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({"format":self.format.id(),"source_pages":self.pages,"editable_text_chars":self.editable_text_chars,
            "rows":self.rows,"cells":self.cells,"page_images":self.page_images,"bytes":self.bytes,
            "fidelity":match self.format { Format::Word=>"simple text reflow; no original fonts or semantic tables",Format::Excel=>"aligned text-object string cells; no formulas or semantic table inference",Format::PowerPoint=>"150 dpi page images; no editable text, vectors or tables"},"original_unchanged":true})
    }
}

fn failure(message: &str) -> EngineError {
    EngineError::Other(message.into())
}
fn cancelled(cancel: &AtomicBool) -> Result<(), EngineError> {
    if cancel.load(Ordering::Relaxed) {
        Err(EngineError::Cancelled)
    } else {
        Ok(())
    }
}

pub(crate) fn xml_text(text: &str) -> Result<String, EngineError> {
    if !text.chars().all(|c| matches!(c, '\t' | '\n' | '\r' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}')) {
        return Err(failure("Office export cannot represent XML 1.0 control characters"));
    }
    Ok(text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;"))
}

#[derive(Debug)]
struct TextPage {
    size: PageSize,
    rows: Vec<Vec<ObjectInfo>>,
}

fn text_page(doc: &Document<'_>, page: usize, format: Format) -> Result<TextPage, EngineError> {
    if !doc.annotations(page)?.is_empty() {
        return Err(failure(
            "Editable Office export refuses annotations and forms; use PowerPoint page images",
        ));
    }
    let size = doc.page_size(page)?;
    if format == Format::Word
        && (size.width_pt > 1584.0
            || size.height_pt > 1584.0
            || size.width_pt < 144.0
            || size.height_pt < 144.0)
    {
        return Err(failure(
            "Word text export requires page dimensions between 2 and 22 inches",
        ));
    }
    let mut objects = doc.page_objects(page)?;
    if objects.len() > 10000 || objects.len() != doc.page_handle(page)?.objects().len() {
        return Err(failure(
            "Editable Office export object limit or malformed page",
        ));
    }
    for object in &objects {
        if object.kind != ObjectKind::Text
            || object.text.is_none()
            || !object.angle_deg.is_finite()
            || object.angle_deg.abs() > 0.1
            || object.opacity.is_none_or(|v| !v.is_finite() || v < 0.999)
            || object.fill_rgb.is_none_or(|rgb| rgb == (255, 255, 255))
            || ![
                object.bounds.left,
                object.bounds.right,
                object.bounds.top,
                object.bounds.bottom,
                object.font_size,
            ]
            .iter()
            .all(|v| v.is_finite())
            || object.bounds.left < -1.0
            || object.bounds.bottom < -1.0
            || object.bounds.right > size.width_pt + 1.0
            || object.bounds.top > size.height_pt + 1.0
            || object.font_size <= 0.0
        {
            return Err(failure("Editable Office export supports visible unrotated text only; use PowerPoint page images"));
        }
        let text = object.text.as_deref().unwrap_or_default();
        xml_text(text)?;
        if text.chars().any(|c| matches!(c, '\u{0590}'..='\u{08ff}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' | '\u{fb1d}'..='\u{fefc}')) || text.contains(['\n', '\r']) {
            return Err(failure("Editable Office export refuses bidirectional or multiline text objects"));
        }
    }
    objects.retain(|o| !o.text.as_deref().unwrap_or_default().is_empty());
    objects.sort_by(|a, b| {
        b.bounds
            .top
            .total_cmp(&a.bounds.top)
            .then(a.bounds.left.total_cmp(&b.bounds.left))
    });
    let mut rows: Vec<Vec<ObjectInfo>> = Vec::new();
    for object in objects {
        if let Some(row) = rows
            .last_mut()
            .filter(|row| (row[0].bounds.top - object.bounds.top).abs() <= 3.0)
        {
            row.push(object);
        } else {
            rows.push(vec![object]);
        }
    }
    for row in &mut rows {
        row.sort_by(|a, b| a.bounds.left.total_cmp(&b.bounds.left));
        if row
            .windows(2)
            .any(|pair| pair[0].bounds.right > pair[1].bounds.left + 1.0)
        {
            return Err(failure("Editable Office export refuses overlapping text"));
        }
    }
    if format == Format::Word {
        let left = rows
            .iter()
            .map(|r| r[0].bounds.left)
            .reduce(f32::min)
            .unwrap_or(0.0);
        if rows.iter().any(|row| {
            row[0].bounds.left - left > 12.0
                || row.windows(2).any(|p| {
                    p[1].bounds.left - p[0].bounds.right > (p[0].font_size * 3.0).max(24.0)
                })
        }) {
            return Err(failure(
                "Word text export refuses columns and irregular indentation",
            ));
        }
    }
    Ok(TextPage { size, rows })
}

impl Document<'_> {
    pub fn export_office(
        &self,
        output: &Path,
        format: Format,
        cancel: &AtomicBool,
        mut progress: impl FnMut(usize, usize),
    ) -> Result<ExportReport, EngineError> {
        cancelled(cancel)?;
        crate::office::validate_local_output(output)?;
        if output.exists() {
            return Err(EngineError::AlreadyExists(output.into()));
        }
        if output
            .extension()
            .and_then(|e| e.to_str())
            .is_none_or(|e| !e.eq_ignore_ascii_case(format.extension()))
        {
            return Err(failure(
                "Office output extension differs from the selected format",
            ));
        }
        if self.page_count() == 0 || self.page_count() > 128 {
            return Err(failure("Office export supports 1 to 128 pages"));
        }
        if !self.properties().can_copy {
            return Err(EngineError::SecurityRestricted);
        }
        if format != Format::PowerPoint {
            let pdf = crate::security::load(&self.snapshot()?, self.password())?;
            if pdf
                .catalog()
                .map_err(crate::security::lopdf_err)?
                .has(b"OCProperties")
            {
                return Err(failure(
                    "Editable Office export refuses optional-content layers",
                ));
            }
            let mut total = 0usize;
            for id in pdf.get_pages().values() {
                let content = pdf
                    .get_page_content_with_limit(*id, 32 * 1024 * 1024 - total)
                    .map_err(crate::security::lopdf_err)?;
                total = total
                    .checked_add(content.len())
                    .ok_or(EngineError::InvalidFormat)?;
                if total > 32 * 1024 * 1024 {
                    return Err(failure("Editable Office export content exceeds 32 MiB"));
                }
                let operations = lopdf::content::Content::decode_strict(&content)
                    .map_err(crate::security::lopdf_err)?;
                if operations.operations.len() > 200000
                    || operations.operations.iter().any(|o| {
                        matches!(o.operator.as_str(), "W" | "W*" | "gs" | "Do" | "cm")
                            || (o.operator == "Tr"
                                && o.operands.first().and_then(|n| n.as_i64().ok()) != Some(0))
                    })
                {
                    return Err(failure("Editable Office export refuses clipping, hidden text, nested content and graphics-state effects"));
                }
            }
        }
        let mut archive = archive::Package::default();
        let mut report = ExportReport {
            format,
            pages: self.page_count(),
            editable_text_chars: 0,
            rows: 0,
            cells: 0,
            page_images: 0,
            bytes: 0,
        };
        if format == Format::PowerPoint {
            parts::presentation(self, &mut archive, cancel, &mut progress)?;
            report.page_images = self.page_count();
        } else {
            let mut pages = Vec::new();
            for page in 0..self.page_count() {
                cancelled(cancel)?;
                let content = text_page(self, page, format)?;
                for row in &content.rows {
                    for object in row {
                        report.editable_text_chars +=
                            object.text.as_deref().unwrap_or_default().chars().count();
                    }
                }
                report.rows += content.rows.len();
                report.cells += content.rows.iter().map(Vec::len).sum::<usize>();
                if report.editable_text_chars > 1_000_000
                    || report.rows > 10000
                    || report.cells > 20000
                {
                    return Err(failure("Editable Office export text/cell limit exceeded"));
                }
                pages.push(content);
                progress(page + 1, self.page_count());
            }
            if report.editable_text_chars == 0 {
                return Err(failure(
                    "No supported editable text; run OCR or use PowerPoint page images",
                ));
            }
            if format == Format::Word {
                parts::word(&pages, &mut archive)?;
            } else {
                parts::workbook(&pages, &mut archive)?;
            }
        }
        cancelled(cancel)?;
        let bytes = archive.finish()?;
        report.bytes = bytes.len();
        cancelled(cancel)?;
        crate::write_new_copy(output, &bytes)?;
        Ok(report)
    }
}

#[cfg(test)]
#[path = "export_office_tests.rs"]
mod tests;
