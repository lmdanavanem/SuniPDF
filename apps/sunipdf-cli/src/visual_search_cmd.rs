use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use sunipdf_core::visual_search::{VisualSearchJob, VisualSearchOptions, MAX_SCAN_PAGES};
use sunipdf_core::{Document, Engine, EngineError, PointRect, Rotation};
use zeroize::Zeroize;

const MAX_SOURCE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_REVIEW_BYTES: u64 = 256 * 1024;

#[derive(Args)]
pub struct VisualSearchArgs {
    #[command(subcommand)]
    command: VisualSearchCommand,
}

#[derive(Args)]
struct Source {
    /// Saved PDF, at most 128 MiB. Passwords use --password-env or --password-stdin.
    file: PathBuf,
    #[arg(skip)]
    password: Option<String>,
}

impl Drop for Source {
    fn drop(&mut self) {
        self.password.zeroize();
    }
}

#[derive(Subcommand)]
enum VisualSearchCommand {
    /// Propose same-size raster matches. No candidate is accepted or counted automatically.
    Inspect {
        #[command(flatten)]
        source: Source,
        /// Physical template page, starting at 1.
        #[arg(long, default_value_t = 1)]
        template_page: usize,
        /// Displayed-page points left,bottom,right,top, with a bottom-left origin.
        #[arg(long, value_parser = parse_rectangle, allow_hyphen_values = true)]
        template_rect: [f32; 4],
        /// Select 1..16 distinct physical pages: 1,3-5 or all for small documents.
        #[arg(long)]
        pages: String,
        /// Quarter-turns relative to the template, e.g. 0,90,180,270.
        #[arg(long, default_value = "0", value_parser = parse_rotations)]
        rotations: QuarterTurns,
        /// Minimum similarity, 50..100. This is not a confidence probability.
        #[arg(long, default_value_t = 90, value_parser = clap::value_parser!(u8).range(50..=100))]
        similarity: u8,
        /// Raster pixels per displayed PDF point; identical for template and searched pages.
        #[arg(long, default_value_t = 1.0, value_parser = positive_scale)]
        scale: f32,
        /// Stop visibly when this many candidates have been found.
        #[arg(long, default_value_t = 256)]
        max_candidates: usize,
        /// Bound matching comparisons; reaching this budget yields an incomplete result.
        #[arg(long, default_value_t = 50_000_000)]
        max_comparisons: u64,
        /// New JSON review file, with no selected candidates; edit it before applying.
        #[arg(long)]
        review_out: Option<PathBuf>,
    },
    /// Rebuild and verify a reviewed search, then create selected counts in a new PDF copy.
    Apply {
        #[command(flatten)]
        source: Source,
        /// Strict review JSON from inspect, with explicitly selected candidate IDs and group.
        #[arg(long)]
        review: PathBuf,
        /// New PDF only. Existing files are refused even with --overwrite.
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Review {
    version: u32,
    raw_file_sha256: String,
    inspection_sha256: String,
    /// Physical page, starting at 1, unlike the core options' explicitly documented page indices.
    template_page: usize,
    template_rect: [f32; 4],
    options: VisualSearchOptions,
    selected_ids: Vec<u32>,
    group: String,
    marker_size: f32,
    author: Option<String>,
    /// Explicit acknowledgement that accepted candidates do not establish a complete count.
    allow_partial: bool,
}

impl VisualSearchArgs {
    fn source(&self) -> &Source {
        match &self.command {
            VisualSearchCommand::Inspect { source, .. }
            | VisualSearchCommand::Apply { source, .. } => source,
        }
    }

    pub fn password_mut(&mut self) -> &mut Option<String> {
        match &mut self.command {
            VisualSearchCommand::Inspect { source, .. }
            | VisualSearchCommand::Apply { source, .. } => &mut source.password,
        }
    }

    pub fn output_path(&self) -> Option<&Path> {
        match &self.command {
            VisualSearchCommand::Inspect { review_out, .. } => review_out.as_deref(),
            VisualSearchCommand::Apply { out, .. } => Some(out),
        }
    }
}

fn invalid(message: &str) -> EngineError {
    EngineError::Other(format!("Visual search: {message}"))
}

fn parse_rectangle(text: &str) -> Result<[f32; 4], String> {
    if text.len() > 256 {
        return Err("template rectangle is too long".into());
    }
    let numbers: Vec<f32> = text
        .split(',')
        .map(|part| {
            part.trim()
                .parse()
                .map_err(|_| "rectangle must contain four numbers")
        })
        .collect::<Result<_, _>>()?;
    let rect: [f32; 4] = numbers
        .try_into()
        .map_err(|_| "rectangle must be left,bottom,right,top")?;
    if !rect.iter().all(|v| v.is_finite()) || rect[0] >= rect[2] || rect[1] >= rect[3] {
        return Err("template rectangle must have finite, ordered bounds".into());
    }
    Ok(rect)
}

#[derive(Debug, Clone)]
struct QuarterTurns(Vec<u16>);

fn parse_rotations(text: &str) -> Result<QuarterTurns, String> {
    if text.len() > 15 {
        return Err("provide one to four distinct quarter-turns".into());
    }
    let mut values = Vec::new();
    for part in text.split(',') {
        let value = part
            .trim()
            .parse::<u16>()
            .map_err(|_| "rotations must be 0,90,180,270")?;
        if ![0, 90, 180, 270].contains(&value) || values.contains(&value) {
            return Err("rotations must be distinct values from 0,90,180,270".into());
        }
        values.push(value);
    }
    Ok(QuarterTurns(values))
}

fn positive_scale(text: &str) -> Result<f32, String> {
    text.parse::<f32>()
        .ok()
        .filter(|v| v.is_finite() && *v > 0.0)
        .ok_or_else(|| "scale must be finite and positive".into())
}

/// Bound page selection before expanding ranges or allocating a collection. The
/// shared general-purpose parser can allocate every repeated page before deduplication.
fn parse_search_pages(spec: &str, count: usize) -> Result<Vec<usize>, EngineError> {
    if spec.len() > 4096 {
        return Err(invalid("page selection exceeds 4096 UTF-8 bytes"));
    }
    if spec.trim().eq_ignore_ascii_case("all") {
        if count == 0 || count > MAX_SCAN_PAGES {
            return Err(invalid("select 1..16 distinct pages"));
        }
        return Ok((0..count).collect());
    }
    let mut pages = BTreeSet::new();
    for part in spec
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        let (first, last) = part.split_once('-').unwrap_or((part, part));
        let first = first.trim().parse::<usize>().map_err(|_| {
            invalid("invalid page selection; use positive pages or ascending ranges")
        })?;
        let last = last.trim().parse::<usize>().map_err(|_| {
            invalid("invalid page selection; use positive pages or ascending ranges")
        })?;
        if first == 0 || last < first {
            return Err(invalid("page ranges must be positive and ascending"));
        }
        if last > count {
            return Err(EngineError::PageOutOfRange(last - 1));
        }
        let length = last
            .checked_sub(first)
            .and_then(|difference| difference.checked_add(1))
            .ok_or_else(|| invalid("page range length overflow"))?;
        if length > MAX_SCAN_PAGES {
            return Err(invalid("select 1..16 distinct pages"));
        }
        let start = first
            .checked_sub(1)
            .ok_or_else(|| invalid("page numbers start at 1"))?;
        for page in start..last {
            if !pages.contains(&page) {
                if pages.len() == MAX_SCAN_PAGES {
                    return Err(invalid("select 1..16 distinct pages"));
                }
                pages.insert(page);
            }
        }
    }
    if pages.is_empty() {
        return Err(invalid("no pages selected"));
    }
    Ok(pages.into_iter().collect())
}

fn read_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>, EngineError> {
    if !std::fs::metadata(path)?.is_file() {
        return Err(invalid("input must be a regular file"));
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(0x800 | 0x20000);
    }
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > maximum {
        return Err(invalid(
            "input exceeds its size limit or is not a regular file",
        ));
    }
    let mut bytes = Vec::new();
    file.take(maximum + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > maximum {
        return Err(invalid("input exceeds its size limit"));
    }
    Ok(bytes)
}

fn hash(bytes: &[u8]) -> String {
    sunipdf_core::plan_zones::source_sha256(bytes)
}

fn revalidate(source: &Source, canonical: &Path, expected: &str) -> Result<(), EngineError> {
    if source.file.canonicalize()? != canonical
        || hash(&read_bounded(canonical, MAX_SOURCE_BYTES)?) != expected
    {
        return Err(invalid("saved source changed; inspect and review again"));
    }
    Ok(())
}

fn rectangle(values: [f32; 4]) -> PointRect {
    PointRect {
        left: values[0],
        bottom: values[1],
        right: values[2],
        top: values[3],
    }
}

fn finish(document: &Document<'_>, job: &mut VisualSearchJob) -> Result<(), EngineError> {
    loop {
        if job.progress().status.is_terminal() {
            return Ok(());
        }
        if job.pending_page().is_some() {
            document.load_visual_search_page(job)?;
        }
        job.step(4096);
    }
}

fn report_value(job: &VisualSearchJob) -> Result<Value, EngineError> {
    serde_json::to_value(job.report()).map_err(|_| invalid("could not serialize inspection"))
}

fn inspection_hash(report: &Value) -> Result<String, EngineError> {
    // PDFium snapshot serialization is not the saved-file identity. The review binds the
    // saved bytes separately and only hashes reproducible search geometry/options/results here.
    let mut stable = report.clone();
    stable
        .as_object_mut()
        .ok_or_else(|| invalid("invalid inspection report"))?
        .remove("source_sha256");
    let bytes =
        serde_json::to_vec(&stable).map_err(|_| invalid("could not fingerprint inspection"))?;
    Ok(hash(&bytes))
}

pub fn run(engine: &Engine, args: VisualSearchArgs) -> Result<Value, EngineError> {
    if let Some(out) = args.output_path() {
        if out.symlink_metadata().is_ok() {
            return Err(EngineError::AlreadyExists(out.to_path_buf()));
        }
    }
    let source = args.source();
    let canonical = source.file.canonicalize()?;
    let original = read_bounded(&canonical, MAX_SOURCE_BYTES)?;
    let raw_file_sha256 = hash(&original);
    let mut document = engine.open_bytes(original, source.password.as_deref(), &canonical)?;
    match &args.command {
        VisualSearchCommand::Inspect {
            template_page,
            template_rect,
            pages,
            rotations,
            similarity,
            scale,
            max_candidates,
            max_comparisons,
            review_out,
            ..
        } => {
            let page = crate::page_index(*template_page, document.page_count())?;
            let pages = parse_search_pages(pages, document.page_count())?;
            let options = VisualSearchOptions {
                pages,
                scale: *scale,
                rotations: rotations
                    .0
                    .iter()
                    .map(|v| Rotation::from_degrees(i32::from(*v)))
                    .collect(),
                sensitivity: f32::from(*similarity) / 100.0,
                max_candidates: *max_candidates,
                max_comparisons: *max_comparisons,
            };
            let mut job =
                document.prepare_visual_search(page, rectangle(*template_rect), options.clone())?;
            finish(&document, &mut job)?;
            let inspection = report_value(&job)?;
            let complete = inspection["progress"]["status"] == "complete";
            let review = Review {
                version: 1,
                raw_file_sha256: raw_file_sha256.clone(),
                inspection_sha256: inspection_hash(&inspection)?,
                template_page: *template_page,
                template_rect: *template_rect,
                options,
                selected_ids: Vec::new(),
                group: "Symbols".into(),
                marker_size: 18.0,
                author: None,
                allow_partial: false,
            };
            revalidate(source, &canonical, &raw_file_sha256)?;
            if let Some(path) = review_out {
                let bytes = serde_json::to_vec_pretty(&review)
                    .map_err(|_| invalid("could not serialize review"))?;
                sunipdf_core::write_new_copy(path, &bytes)?;
            }
            Ok(
                json!({"operation":"inspect", "complete":complete, "published":false,
                "raw_file_sha256":raw_file_sha256, "source_unchanged":true,
                "inspection":inspection, "review":review, "review_out":review_out,
                "inspection_page_index_base":0, "review_template_page_index_base":1,
                "similarity_is_probability":false, "accepted_count":0}),
            )
        }
        VisualSearchCommand::Apply { review, out, .. } => {
            let review: Review =
                serde_json::from_slice(&read_bounded(review, MAX_REVIEW_BYTES)?)
                    .map_err(|error| invalid(&format!("invalid strict review JSON: {error}")))?;
            if review.version != 1 || review.raw_file_sha256 != raw_file_sha256 {
                return Err(invalid("review version or saved source identity differs"));
            }
            let page = crate::page_index(review.template_page, document.page_count())?;
            let mut job = document.prepare_visual_search(
                page,
                rectangle(review.template_rect),
                review.options,
            )?;
            finish(&document, &mut job)?;
            let inspection = report_value(&job)?;
            if inspection_hash(&inspection)? != review.inspection_sha256 {
                return Err(invalid(
                    "search results or options changed; inspect and review again",
                ));
            }
            let complete = inspection["progress"]["status"] == "complete";
            if !complete && !review.allow_partial {
                return Err(invalid("search is incomplete; explicitly acknowledge partial coverage before applying selected counts"));
            }
            let prepared = document.prepare_visual_search_counts(
                &job,
                &review.selected_ids,
                &review.group,
                review.marker_size,
                review.author.as_deref(),
            )?;
            let count = document.apply_visual_search_counts(&prepared)?;
            let candidate = document.snapshot()?;
            revalidate(source, &canonical, &raw_file_sha256)?;
            sunipdf_core::write_new_copy(out, &candidate)?;
            Ok(
                json!({"operation":"apply", "complete":complete, "search_complete":complete,
                "published":true, "out":out, "accepted_count":count,
                "raw_file_sha256":raw_file_sha256, "output_sha256":hash(&candidate),
                "source_unchanged":true, "inspection":inspection,
                "inspection_page_index_base":0, "similarity_is_probability":false}),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_selection_is_bounded_before_expansion_and_preserves_sorted_unique_semantics() {
        assert_eq!(
            parse_search_pages("5, 2-4, 3, 1,,5", 5).unwrap(),
            vec![0, 1, 2, 3, 4]
        );
        let sixteen: Vec<usize> = (0..16).collect();
        assert_eq!(parse_search_pages("  ALL  ", 16).unwrap(), sixteen);
        assert_eq!(
            parse_search_pages("1-16,1-16,8-12", 32768).unwrap(),
            sixteen
        );
        assert_eq!(parse_search_pages("1-15,15-16", 32768).unwrap(), sixteen);
        assert!(
            parse_search_pages("2-17,1", 32768).is_err(),
            "the union must not grow to 17 pages"
        );
        for count in [0, 17, 32768, usize::MAX] {
            assert!(parse_search_pages("all", count).is_err());
        }
        for spec in ["1-17", "1-32768"] {
            assert!(parse_search_pages(spec, 32768).is_err());
        }
        assert!(parse_search_pages(&"1-32768,".repeat(400), 32768).is_err());
        assert!(parse_search_pages(&format!("1-{}", usize::MAX), usize::MAX).is_err());
        assert_eq!(
            parse_search_pages(&format!("{}-{}", usize::MAX, usize::MAX), usize::MAX).unwrap(),
            vec![usize::MAX - 1]
        );
        assert!(parse_search_pages(&format!("{}0", usize::MAX), usize::MAX).is_err());
        assert!(parse_search_pages(&format!("{}-1", usize::MAX), usize::MAX).is_err());
        for spec in [
            "", ",", "0", "0-1", "2-1", "1-2-3", "invalid", "-1", "1,0", "32769",
        ] {
            assert!(parse_search_pages(spec, 32768).is_err(), "{spec}");
        }
        assert!(parse_search_pages("1", 0).is_err());
        let maximum_spec = format!("1{}", ",".repeat(4095));
        assert_eq!(parse_search_pages(&maximum_spec, 1).unwrap(), vec![0]);
        assert!(parse_search_pages(&(maximum_spec + ","), 1).is_err());
    }

    #[test]
    fn numeric_inputs_are_bounded_and_explicit() {
        assert_eq!(
            parse_rectangle("10,20,40,60").unwrap(),
            [10., 20., 40., 60.]
        );
        for invalid in ["1,2,3", "0,0,NaN,5", "0,0,inf,5", "1,1,1,2", "2,1,1,2"] {
            assert!(parse_rectangle(invalid).is_err());
        }
        assert_eq!(
            parse_rotations("0,90,180,270").unwrap().0,
            [0, 90, 180, 270]
        );
        for invalid in ["", "90,90", "45", "360", "0,90,180,270,0"] {
            assert!(parse_rotations(invalid).is_err());
        }
        for invalid in ["NaN", "inf", "-1", "0"] {
            assert!(positive_scale(invalid).is_err());
        }
    }
}
