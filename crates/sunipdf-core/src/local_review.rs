//! Read-only, bounded inspection of explicitly selected local files.

use crate::{Document, Engine, EngineError, RenderedPage};
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

pub const MAX_FILES: usize = 64;
pub const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_PAGE_CHARS: usize = 250_000;
pub const MAX_SEARCH_PAGES: usize = 10_000;
pub const MAX_HIT_PAGES: usize = 2_000;
pub const PREVIEW_SIDE: u32 = 768;

fn read_pdf(path: &Path) -> Result<Vec<u8>, EngineError> {
    let file = std::fs::File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(EngineError::Other("Choose a regular PDF file".into()));
    }
    if metadata.len() > MAX_FILE_BYTES {
        return Err(EngineError::Other(
            "Local review input exceeds 64 MiB".into(),
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(EngineError::Other(
            "Local review input exceeds 64 MiB".into(),
        ));
    }
    Ok(bytes)
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn check_permission(doc: &Document<'_>) -> Result<(), EngineError> {
    if !doc.properties().can_copy {
        return Err(EngineError::SecurityRestricted);
    }
    Ok(())
}

fn page_text(doc: &Document<'_>, index: usize) -> Result<String, EngineError> {
    let page_index = pdfium_render::prelude::PdfPageIndex::try_from(index)
        .map_err(|_| EngineError::PageOutOfRange(index))?;
    let page = doc.inner.pages().get(page_index)?;
    if page.text()?.len() > MAX_PAGE_CHARS as i32 {
        return Err(EngineError::Other(
            "Local review page exceeds 250000 characters".into(),
        ));
    }
    doc.page_text(index)
}

#[derive(Debug)]
pub struct PageHit {
    /// Physical, one-based page number.
    pub page: usize,
    pub occurrences: usize,
    pub snippet: String,
}

#[derive(Debug)]
pub struct FileSearch {
    pub path: PathBuf,
    pub source_sha256: Option<String>,
    pub status: &'static str,
    pub error: Option<EngineError>,
    pub page_count: Option<usize>,
    pub scanned_pages: usize,
    pub hits: Vec<PageHit>,
}

#[derive(Debug)]
pub struct SearchReport {
    pub query: String,
    pub match_case: bool,
    pub files: Vec<FileSearch>,
}

impl SearchReport {
    pub fn complete(&self) -> bool {
        self.files.iter().all(|file| file.status == "complete")
    }

    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "query": self.query, "match_case": self.match_case, "complete": self.complete(),
            "files": self.files.iter().map(|file| serde_json::json!({
                "path": file.path, "source_sha256": file.source_sha256, "status": file.status,
                "error": file.error.as_ref().map(|e| e.code()),
                "message": file.error.as_ref().map(|e| e.to_string()),
                "page_count": file.page_count, "scanned_pages": file.scanned_pages,
                "hits": file.hits.iter().map(|hit| serde_json::json!({
                    "page": hit.page, "occurrences": hit.occurrences, "snippet": hit.snippet,
                })).collect::<Vec<_>>()
            })).collect::<Vec<_>>()
        })
    }
}

fn matches(text: &str, query: &str, match_case: bool) -> Option<(usize, String)> {
    let haystack = if match_case {
        text.to_owned()
    } else {
        text.to_lowercase()
    };
    let needle = if match_case {
        query.to_owned()
    } else {
        query.to_lowercase()
    };
    let first = haystack.find(&needle)?;
    let count = haystack.match_indices(&needle).count();
    // Lowercasing can expand a character; map the first result back to the original text.
    let mut folded = 0;
    let mut original = 0;
    for (offset, ch) in text.char_indices() {
        if folded >= first {
            original = offset;
            break;
        }
        folded += if match_case {
            ch.len_utf8()
        } else {
            ch.to_lowercase().map(char::len_utf8).sum()
        };
        original = offset;
    }
    let start = text[..original]
        .char_indices()
        .rev()
        .nth(60)
        .map_or(0, |(i, _)| i);
    let snippet = text[start..]
        .chars()
        .take(240)
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect();
    Some((count, snippet))
}

/// No directory traversal, indexing database, background OCR or network activity.
/// Cancellation is observed between files and native page operations.
pub fn search_files(
    engine: &Engine,
    paths: &[PathBuf],
    query: &str,
    match_case: bool,
    password: Option<&str>,
    cancel: &AtomicBool,
    mut progress: impl FnMut(usize, usize, usize, usize),
) -> Result<SearchReport, EngineError> {
    if paths.is_empty()
        || paths.len() > MAX_FILES
        || query.trim().is_empty()
        || query.chars().count() > 256
        || query.contains('\0')
    {
        return Err(EngineError::Other(
            "Choose 1 to 64 files and a query of 1 to 256 characters".into(),
        ));
    }
    let mut report = SearchReport {
        query: query.into(),
        match_case,
        files: Vec::new(),
    };
    let mut scanned = 0;
    let mut hit_pages = 0;
    for (index, path) in paths.iter().enumerate() {
        let mut file = FileSearch {
            path: path.clone(),
            source_sha256: None,
            status: "complete",
            error: None,
            page_count: None,
            scanned_pages: 0,
            hits: Vec::new(),
        };
        if cancel.load(Ordering::Relaxed) {
            file.status = "cancelled";
        } else if scanned >= MAX_SEARCH_PAGES || hit_pages >= MAX_HIT_PAGES {
            file.status = "limit_reached";
        } else {
            let result = (|| {
                let bytes = read_pdf(path)?;
                file.source_sha256 = Some(digest(&bytes));
                let doc = engine.open_bytes(bytes, password, path)?;
                check_permission(&doc)?;
                file.page_count = Some(doc.page_count());
                for page in 0..doc.page_count() {
                    if cancel.load(Ordering::Relaxed) {
                        file.status = "cancelled";
                        break;
                    }
                    if scanned >= MAX_SEARCH_PAGES || hit_pages >= MAX_HIT_PAGES {
                        file.status = "limit_reached";
                        break;
                    }
                    let text = page_text(&doc, page)?;
                    if let Some((occurrences, snippet)) = matches(&text, query, match_case) {
                        file.hits.push(PageHit {
                            page: page + 1,
                            occurrences,
                            snippet,
                        });
                        hit_pages += 1;
                    }
                    file.scanned_pages += 1;
                    scanned += 1;
                    progress(index, paths.len(), file.scanned_pages, doc.page_count());
                }
                Ok::<_, EngineError>(())
            })();
            if let Err(error) = result {
                file.status = "error";
                file.error = Some(error);
            }
        }
        report.files.push(file);
        progress(index + 1, paths.len(), 0, 0);
    }
    Ok(report)
}

#[derive(Debug, Clone, Copy)]
pub struct CompareOptions {
    pub left_start: usize,
    pub right_start: usize,
    pub max_pages: usize,
}
impl Default for CompareOptions {
    fn default() -> Self {
        Self {
            left_start: 1,
            right_start: 1,
            max_pages: 200,
        }
    }
}

#[derive(Debug)]
pub struct PageDifference {
    pub left_page: Option<usize>,
    pub right_page: Option<usize>,
    pub text_changed: bool,
    pub geometry_changed: bool,
    pub changed_pixels: usize,
    pub compared_pixels: usize,
    pub bounds: Option<[u32; 4]>,
    pub no_text_on_either_page: bool,
}
impl PageDifference {
    pub fn changed(&self) -> bool {
        self.left_page.is_none()
            || self.right_page.is_none()
            || self.text_changed
            || self.geometry_changed
            || self.changed_pixels > 0
    }
}

#[derive(Debug)]
pub struct Comparison {
    pub left: PathBuf,
    pub right: PathBuf,
    pub left_sha256: String,
    pub right_sha256: String,
    pub options: CompareOptions,
    pub aligned_pages: usize,
    pub cancelled: bool,
    pub pages: Vec<PageDifference>,
}
impl Comparison {
    pub fn complete(&self) -> bool {
        !self.cancelled && self.pages.len() == self.aligned_pages
    }
    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "left": self.left, "right": self.right, "left_sha256": self.left_sha256,
            "right_sha256": self.right_sha256, "alignment": "physical_page_offsets",
            "left_start": self.options.left_start, "right_start": self.options.right_start,
            "complete": self.complete(), "cancelled": self.cancelled, "aligned_pages": self.aligned_pages,
            "preview_long_side": PREVIEW_SIDE, "channel_tolerance": 16,
            "pages": self.pages.iter().map(|page| serde_json::json!({
                "left_page": page.left_page, "right_page": page.right_page,
                "changed": page.changed(), "text_changed": page.text_changed,
                "geometry_changed": page.geometry_changed, "changed_pixels": page.changed_pixels,
                "compared_pixels": page.compared_pixels, "pixel_bounds": page.bounds,
                "no_text_on_either_page": page.no_text_on_either_page,
            })).collect::<Vec<_>>()
        })
    }
}

pub struct ComparisonPreview {
    pub left: Option<RenderedPage>,
    pub right: Option<RenderedPage>,
    pub difference: RenderedPage,
}

fn pair(
    left: &Document<'_>,
    right: &Document<'_>,
    lp: Option<usize>,
    rp: Option<usize>,
) -> Result<(PageDifference, ComparisonPreview), EngineError> {
    let ls = lp.map(|i| left.page_size(i)).transpose()?;
    let rs = rp.map(|i| right.page_size(i)).transpose()?;
    let longest = ls
        .iter()
        .chain(rs.iter())
        .map(|s| s.width_pt.max(s.height_pt))
        .fold(0.0_f32, f32::max);
    if !longest.is_finite() || longest <= 0.0 {
        return Err(EngineError::RenderTooLarge);
    }
    let scale = PREVIEW_SIDE as f32 / longest;
    let a = lp.map(|i| left.render_page(i, scale)).transpose()?;
    let b = rp.map(|i| right.render_page(i, scale)).transpose()?;
    let lt = lp
        .map(|i| page_text(left, i))
        .transpose()?
        .unwrap_or_default();
    let rt = rp
        .map(|i| page_text(right, i))
        .transpose()?
        .unwrap_or_default();
    let (width, height) = a
        .iter()
        .chain(b.iter())
        .fold((0, 0), |(w, h), img| (w.max(img.width), h.max(img.height)));
    if width == 0 || height == 0 || width > PREVIEW_SIDE + 1 || height > PREVIEW_SIDE + 1 {
        return Err(EngineError::RenderTooLarge);
    }
    let pixel = |image: &Option<RenderedPage>, x: u32, y: u32| -> [u8; 3] {
        image
            .as_ref()
            .filter(|im| x < im.width && y < im.height)
            .map_or([255; 3], |im| {
                let i = ((y * im.width + x) * 4) as usize;
                [im.rgba[i], im.rgba[i + 1], im.rgba[i + 2]]
            })
    };
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    let mut changed_pixels = 0;
    let mut bounds = [width, height, 0, 0];
    for y in 0..height {
        for x in 0..width {
            let p = pixel(&a, x, y);
            let q = pixel(&b, x, y);
            if p.iter().zip(q).any(|(p, q)| p.abs_diff(q) > 16) {
                changed_pixels += 1;
                bounds = [
                    bounds[0].min(x),
                    bounds[1].min(y),
                    bounds[2].max(x + 1),
                    bounds[3].max(y + 1),
                ];
                rgba.extend_from_slice(&[220, 35, 65, 255]);
            } else {
                let gray = (u16::from(p[0]) + u16::from(p[1]) + u16::from(p[2])) / 3;
                let faded = 200 + (gray * 55 / 255) as u8;
                rgba.extend_from_slice(&[faded, faded, faded, 255]);
            }
        }
    }
    let geometry_changed = match (ls, rs) {
        (Some(a), Some(b)) => {
            (a.width_pt - b.width_pt).abs() > 0.01 || (a.height_pt - b.height_pt).abs() > 0.01
        }
        _ => true,
    };
    Ok((
        PageDifference {
            left_page: lp.map(|i| i + 1),
            right_page: rp.map(|i| i + 1),
            text_changed: lt != rt,
            geometry_changed,
            changed_pixels,
            compared_pixels: (width * height) as usize,
            bounds: (changed_pixels > 0).then_some(bounds),
            no_text_on_either_page: lt.trim().is_empty() && rt.trim().is_empty(),
        },
        ComparisonPreview {
            left: a,
            right: b,
            difference: RenderedPage {
                page_index: 0,
                width,
                height,
                rgba,
            },
        },
    ))
}

pub fn compare_files(
    engine: &Engine,
    left: &Path,
    right: &Path,
    options: CompareOptions,
    password: Option<&str>,
    cancel: &AtomicBool,
    mut progress: impl FnMut(usize, usize),
) -> Result<Comparison, EngineError> {
    if options.left_start == 0
        || options.right_start == 0
        || !(1..=500).contains(&options.max_pages)
    {
        return Err(EngineError::Other(
            "Comparison uses one-based start pages and 1 to 500 pairs".into(),
        ));
    }
    let a = read_pdf(left)?;
    let b = read_pdf(right)?;
    let mut report = Comparison {
        left: left.into(),
        right: right.into(),
        left_sha256: digest(&a),
        right_sha256: digest(&b),
        options,
        aligned_pages: 0,
        cancelled: false,
        pages: Vec::new(),
    };
    let a = engine.open_bytes(a, password, left)?;
    let b = engine.open_bytes(b, password, right)?;
    check_permission(&a)?;
    check_permission(&b)?;
    if options.left_start > a.page_count() || options.right_start > b.page_count() {
        return Err(EngineError::PageOutOfRange(
            options.left_start.max(options.right_start) - 1,
        ));
    }
    report.aligned_pages =
        (a.page_count() - options.left_start + 1).max(b.page_count() - options.right_start + 1);
    for i in 0..report.aligned_pages.min(options.max_pages) {
        if cancel.load(Ordering::Relaxed) {
            report.cancelled = true;
            break;
        }
        let lp = options.left_start - 1 + i;
        let rp = options.right_start - 1 + i;
        let (result, _) = pair(
            &a,
            &b,
            (lp < a.page_count()).then_some(lp),
            (rp < b.page_count()).then_some(rp),
        )?;
        report.pages.push(result);
        progress(i + 1, report.aligned_pages.min(options.max_pages));
    }
    Ok(report)
}

/// Reopen exact recorded bytes; refuse to display a preview for a changed source.
pub fn comparison_preview(
    engine: &Engine,
    report: &Comparison,
    index: usize,
    password: Option<&str>,
) -> Result<ComparisonPreview, EngineError> {
    let page = report
        .pages
        .get(index)
        .ok_or(EngineError::PageOutOfRange(index))?;
    let a = read_pdf(&report.left)?;
    let b = read_pdf(&report.right)?;
    if digest(&a) != report.left_sha256 || digest(&b) != report.right_sha256 {
        return Err(EngineError::Other(
            "A source changed; run the comparison again".into(),
        ));
    }
    let a = engine.open_bytes(a, password, &report.left)?;
    let b = engine.open_bytes(b, password, &report.right)?;
    check_permission(&a)?;
    check_permission(&b)?;
    pair(
        &a,
        &b,
        page.left_page.map(|p| p - 1),
        page.right_page.map(|p| p - 1),
    )
    .map(|(_, preview)| preview)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snippets_keep_unicode_case_and_count_literal_non_overlapping_matches() {
        assert_eq!(
            matches("ÉTÉ İ TEST test", "test", false),
            Some((2, "ÉTÉ İ TEST test".into()))
        );
        assert_eq!(matches("a.b axb a.b", "a.b", true).unwrap().0, 2);
        assert!(matches("Été", "été", true).is_none());
        let long = format!("{} Needle {}", "é".repeat(500), "x".repeat(500));
        let (_, snippet) = matches(&long, "needle", false).unwrap();
        assert!(snippet.contains("Needle"));
        assert!(snippet.chars().count() <= 240);
    }
}
