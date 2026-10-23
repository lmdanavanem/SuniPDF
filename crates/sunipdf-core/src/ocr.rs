//! Offline OCR using bundled Windows or Ubuntu system Tesseract and an invisible PDF text layer.

use crate::{Document, EngineError, PageSize, PointRect, RenderedPage};
use pdfium_render::prelude::*;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const MAX_IMAGE_PIXELS: f32 = 16_000_000.0;
const MAX_TSV_BYTES: u64 = 16 * 1024 * 1024;
const MAX_STDERR_BYTES: u64 = 1024 * 1024;
const MAX_WORDS: usize = 50_000;
const PAGE_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OcrLanguage {
    English,
    French,
    #[default]
    EnglishFrench,
}

impl OcrLanguage {
    fn code(self) -> &'static str {
        match self {
            Self::English => "eng",
            Self::French => "fra",
            Self::EnglishFrench => "eng+fra",
        }
    }
}

#[derive(Debug, Clone)]
pub struct OcrOptions {
    pub pages: Vec<usize>,
    pub language: OcrLanguage,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OcrReport {
    pub recognized_pages: usize,
    pub skipped_pages: usize,
    pub words: usize,
}

#[derive(Debug, PartialEq)]
struct Word {
    text: String,
    rect: PointRect,
}

fn failure(message: &str) -> EngineError {
    EngineError::Other(message.into())
}

fn check_cancel(cancel: &AtomicBool) -> Result<(), EngineError> {
    if cancel.load(Ordering::Relaxed) {
        Err(EngineError::Cancelled)
    } else {
        Ok(())
    }
}

/// Search the application directory; debug builds also accept an explicit development override.
#[cfg(windows)]
pub fn runtime_dir() -> Result<PathBuf, EngineError> {
    let mut candidates = Vec::new();
    #[cfg(debug_assertions)]
    if let Some(path) = std::env::var_os("SUNIPDF_OCR_DIR") {
        candidates.push(PathBuf::from(path));
    }
    if let Some(parent) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
    {
        candidates.push(parent.join("ocr"));
        // Flat licenses payloads remain compatible with the 0.8.x portable updater allowlist.
        candidates.push(parent.join("licenses"));
    }
    #[cfg(debug_assertions)]
    candidates.push(
        Path::new(env!("CARGO_MANIFEST_DIR")).join(if cfg!(target_arch = "aarch64") {
            "../../third_party/ocr/win-arm64"
        } else {
            "../../third_party/ocr/win-x64"
        }),
    );
    candidates
        .into_iter()
        .find(|dir| {
            dir.join("tesseract.exe").is_file()
                && data_dir(dir).join("eng.traineddata").is_file()
                && data_dir(dir).join("fra.traineddata").is_file()
        })
        .ok_or_else(|| failure("bundled OCR runtime is missing"))?
        .canonicalize()
        .map(|path| ordinary_path(&path))
        .map_err(EngineError::Io)
}

#[cfg(target_os = "linux")]
const SYSTEM_TESSDATA_DIRS: [&str; 3] = [
    "/usr/share/tesseract-ocr/5/tessdata",
    "/usr/share/tesseract-ocr/4.00/tessdata",
    "/usr/share/tessdata",
];

/// Ubuntu packages supply an absolute executable and both offline language datasets.
#[cfg(target_os = "linux")]
pub fn runtime_dir() -> Result<PathBuf, EngineError> {
    use std::os::unix::fs::PermissionsExt;
    let directory = PathBuf::from("/usr/bin");
    let executable = runtime_executable(&directory);
    let runnable = std::fs::metadata(executable)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0);
    if runnable && has_language_data(&data_dir(&directory)) {
        Ok(directory)
    } else {
        Err(failure(
            "system OCR requires /usr/bin/tesseract and the English and French language packages",
        ))
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn runtime_dir() -> Result<PathBuf, EngineError> {
    Err(failure("OCR is unavailable on this platform"))
}

fn runtime_executable(runtime: &Path) -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        let _ = runtime;
        PathBuf::from("/usr/bin/tesseract")
    }
    #[cfg(not(target_os = "linux"))]
    runtime.join("tesseract.exe")
}

#[cfg(target_os = "linux")]
fn has_language_data(directory: &Path) -> bool {
    ["eng.traineddata", "fra.traineddata"]
        .iter()
        .all(|name| directory.join(name).is_file())
}

#[cfg(target_os = "linux")]
fn find_system_data<'a>(directories: impl IntoIterator<Item = &'a Path>) -> Option<PathBuf> {
    directories
        .into_iter()
        .find(|directory| directory.is_absolute() && has_language_data(directory))
        .map(Path::to_path_buf)
}

// Tesseract appends ordinary separators to its C stdio paths; Windows verbatim prefixes break them.
#[cfg(windows)]
fn ordinary_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        let mut components = path.components();
        if let Some(Component::Prefix(prefix)) = components.next() {
            let mut plain = match prefix.kind() {
                Prefix::VerbatimDisk(drive) => PathBuf::from(format!("{}:", char::from(drive))),
                Prefix::VerbatimUNC(server, share) => {
                    let mut plain = PathBuf::from(r"\\");
                    plain.push(server);
                    plain.push(share);
                    plain
                }
                _ => return path.to_owned(),
            };
            plain.extend(components);
            return plain;
        }
    }
    path.to_owned()
}

#[cfg(not(target_os = "linux"))]
fn data_dir(runtime: &Path) -> PathBuf {
    let data = runtime.join("tessdata");
    if data.is_dir() {
        data
    } else {
        runtime.to_path_buf()
    }
}

#[cfg(target_os = "linux")]
fn data_dir(_runtime: &Path) -> PathBuf {
    find_system_data(SYSTEM_TESSDATA_DIRS.iter().map(Path::new))
        .unwrap_or_else(|| PathBuf::from(SYSTEM_TESSDATA_DIRS[0]))
}

fn render_scale(size: PageSize) -> Result<f32, EngineError> {
    let (w, h) = (size.width_pt, size.height_pt);
    if !(w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0) {
        return Err(EngineError::RenderTooLarge);
    }
    let scale = (300.0f32 / 72.0)
        .min((MAX_IMAGE_PIXELS / (w * h)).sqrt())
        .min(8192.0 / w)
        .min(8192.0 / h);
    if !scale.is_finite() || scale < 1.0 {
        return Err(EngineError::RenderTooLarge);
    }
    Ok(scale)
}

fn write_pnm(path: &Path, image: &RenderedPage) -> Result<(), EngineError> {
    if image.rgba.len() != image.width as usize * image.height as usize * 4 {
        return Err(failure("invalid OCR image"));
    }
    let mut out = BufWriter::new(std::fs::File::create(path)?);
    write!(out, "P6\n{} {}\n255\n", image.width, image.height)?;
    for pixel in image.rgba.as_chunks::<4>().0 {
        // Composite transparency onto white before passing the scan to OCR.
        let alpha = u16::from(pixel[3]);
        let rgb = [pixel[0], pixel[1], pixel[2]]
            .map(|c| ((u16::from(c) * alpha + 255 * (255 - alpha) + 127) / 255) as u8);
        out.write_all(&rgb)?;
    }
    out.flush()?;
    Ok(())
}

struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn recognize(
    runtime: &Path,
    image: &RenderedPage,
    size: PageSize,
    language: OcrLanguage,
    cancel: &AtomicBool,
) -> Result<Vec<Word>, EngineError> {
    check_cancel(cancel)?;
    let temp = tempfile::Builder::new().prefix("sunipdf-ocr-").tempdir()?;
    let input = temp.path().join("page.ppm");
    let output = temp.path().join("words.tsv");
    let stderr = temp.path().join("stderr.txt");
    write_pnm(&input, image)?;
    check_cancel(cancel)?;
    let mut command = std::process::Command::new(runtime_executable(runtime));
    command
        .arg(&input)
        .arg("stdout")
        .args(["-l", language.code(), "--tessdata-dir"])
        .arg(data_dir(runtime))
        .args(["--dpi", "300", "--psm", "3", "-c", "tessedit_create_tsv=1"])
        .env("OMP_THREAD_LIMIT", "2")
        .current_dir(temp.path())
        .stdin(Stdio::null())
        .stdout(std::fs::File::create(&output)?)
        .stderr(std::fs::File::create(&stderr)?);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = Running(command.spawn()?);
    let started = Instant::now();
    loop {
        check_cancel(cancel)?;
        if started.elapsed() > PAGE_TIMEOUT {
            return Err(failure("OCR page exceeded the two-minute time limit"));
        }
        if std::fs::metadata(&output)?.len() > MAX_TSV_BYTES
            || std::fs::metadata(&stderr)?.len() > MAX_STDERR_BYTES
        {
            return Err(failure("OCR output exceeds limits"));
        }
        if let Some(status) = child.0.try_wait()? {
            if !status.success() {
                return Err(failure("OCR engine failed to recognize the page"));
            }
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    check_cancel(cancel)?;
    let mut tsv = String::new();
    std::fs::File::open(output)?
        .take(MAX_TSV_BYTES + 1)
        .read_to_string(&mut tsv)?;
    if tsv.len() as u64 > MAX_TSV_BYTES {
        return Err(failure("OCR output exceeds limits"));
    }
    parse_tsv(&tsv, image.width, image.height, size)
}

fn parse_tsv(tsv: &str, width: u32, height: u32, size: PageSize) -> Result<Vec<Word>, EngineError> {
    let mut lines = tsv.lines();
    if lines.next() != Some("level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext") || width == 0 || height == 0 {
        return Err(failure("invalid OCR output"));
    }
    let mut words = Vec::new();
    for line in lines {
        let columns: Vec<_> = line.splitn(12, '\t').collect();
        if columns.len() != 12 {
            return Err(failure("invalid OCR output"));
        }
        if columns[0] != "5" || columns[11].trim().is_empty() {
            continue;
        }
        let number = |index: usize| {
            columns[index]
                .parse::<u32>()
                .map_err(|_| failure("invalid OCR word bounds"))
        };
        let (x, y, w, h) = (number(6)?, number(7)?, number(8)?, number(9)?);
        let confidence = columns[10]
            .parse::<f32>()
            .map_err(|_| failure("invalid OCR confidence"))?;
        if !confidence.is_finite()
            || !(0.0..=100.0).contains(&confidence)
            || w == 0
            || h == 0
            || x.checked_add(w).is_none_or(|right| right > width)
            || y.checked_add(h).is_none_or(|bottom| bottom > height)
            || columns[11].len() > 4096
            || columns[11].chars().any(char::is_control)
            || words.len() >= MAX_WORDS
        {
            return Err(failure("invalid or excessive OCR words"));
        }
        words.push(Word {
            text: columns[11].to_owned(),
            rect: PointRect {
                left: x as f32 * size.width_pt / width as f32,
                right: (x + w) as f32 * size.width_pt / width as f32,
                bottom: (height - y - h) as f32 * size.height_pt / height as f32,
                top: (height - y) as f32 * size.height_pt / height as f32,
            },
        });
    }
    Ok(words)
}

impl Document<'_> {
    /// Recognize image-only pages locally. A failed or cancelled run leaves this document unchanged.
    pub fn ocr(
        &mut self,
        options: &OcrOptions,
        cancel: &AtomicBool,
        mut progress: impl FnMut(usize, usize),
    ) -> Result<OcrReport, EngineError> {
        if !self.properties().can_modify {
            return Err(EngineError::SecurityRestricted);
        }
        let mut pages = options.pages.clone();
        pages.sort_unstable();
        pages.dedup();
        if pages.is_empty() {
            return Err(failure("no OCR pages selected"));
        }
        for &page in &pages {
            self.page_size(page)?;
        }
        check_cancel(cancel)?;
        let runtime = runtime_dir()?;
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
        let mut report = OcrReport::default();
        for (done, &page) in pages.iter().enumerate() {
            check_cancel(cancel)?;
            if !scratch.page_text(page)?.trim().is_empty() {
                report.skipped_pages += 1;
            } else {
                let size = scratch.page_size(page)?;
                let image = scratch.render_page(page, render_scale(size)?)?;
                let words = recognize(&runtime, &image, size, options.language, cancel)?;
                if !words.is_empty() {
                    scratch.add_ocr_words(page, &words)?;
                    report.recognized_pages += 1;
                    report.words += words.len();
                }
            }
            progress(done + 1, pages.len());
        }
        check_cancel(cancel)?;
        if report.words > 0 {
            self.inner = scratch.inner;
            self.font_cache.clear();
        }
        Ok(report)
    }

    fn add_ocr_words(&mut self, page_index: usize, words: &[Word]) -> Result<(), EngineError> {
        let geo = self.page_geometry(page_index)?;
        let font = self.inner.fonts_mut().helvetica();
        let mut page = self.page_handle(page_index)?;
        for word in words {
            let mut text =
                PdfPageTextObject::new(&self.inner, &word.text, font, PdfPoints::new(12.0))?;
            text.set_render_mode(PdfPageTextRenderMode::Invisible)?;
            let bounds = text.bounds()?;
            if bounds.width().value <= 0.0 || bounds.height().value <= 0.0 {
                return Err(failure("OCR word has no font bounds"));
            }
            let sx = (word.rect.right - word.rect.left) / bounds.width().value;
            let sy = (word.rect.top - word.rect.bottom) / bounds.height().value;
            let x = word.rect.left - bounds.left().value * sx;
            let y = word.rect.bottom - bounds.bottom().value * sy;
            let m = geo.upright_matrix(x, y);
            text.transform(m[0] * sx, m[1] * sx, m[2] * sy, m[3] * sy, m[4], m[5])?;
            let object = page.objects_mut().add_text_object(text)?;
            if object
                .as_text_object()
                .is_none_or(|t| t.text() != word.text)
            {
                return Err(EngineError::MissingGlyphs(word.text.clone()));
            }
        }
        crate::edit::regenerate_content(&mut page)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const HEADER: &str = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n";
    fn size() -> PageSize {
        PageSize {
            width_pt: 600.0,
            height_pt: 800.0,
        }
    }

    #[cfg(windows)]
    #[test]
    fn development_and_compatible_portable_data_layouts_are_supported() {
        let temp = tempfile::tempdir().unwrap();
        assert_eq!(data_dir(temp.path()), temp.path());
        std::fs::create_dir(temp.path().join("tessdata")).unwrap();
        assert_eq!(data_dir(temp.path()), temp.path().join("tessdata"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn system_ocr_requires_both_languages_and_never_resolves_a_path_executable() {
        let temp = tempfile::tempdir().unwrap();
        let incomplete = temp.path().join("incomplete");
        let complete = temp.path().join("complete");
        for directory in [&incomplete, &complete] {
            std::fs::create_dir(directory).unwrap();
            std::fs::write(directory.join("eng.traineddata"), b"fixture").unwrap();
        }
        std::fs::write(complete.join("fra.traineddata"), b"fixture").unwrap();
        assert_eq!(
            find_system_data([incomplete.as_path(), complete.as_path()]),
            Some(complete)
        );
        assert!(find_system_data([incomplete.as_path(), Path::new("tessdata")]).is_none());
        assert_eq!(
            runtime_executable(Path::new("untrusted-runtime")),
            PathBuf::from("/usr/bin/tesseract")
        );
    }

    #[cfg(windows)]
    #[test]
    fn ocr_paths_preserve_unicode_and_unc_without_verbatim_prefixes() {
        assert_eq!(
            ordinary_path(Path::new(r"\\?\C:\Users\Émanuel\SuniPDF")),
            PathBuf::from(r"C:\Users\Émanuel\SuniPDF")
        );
        assert_eq!(
            ordinary_path(Path::new(r"\\?\UNC\server\share\SuniPDF")),
            PathBuf::from(r"\\server\share\SuniPDF")
        );
        assert_eq!(
            ordinary_path(Path::new(r"C:\SuniPDF")),
            PathBuf::from(r"C:\SuniPDF")
        );
    }

    #[test]
    fn tsv_maps_top_left_pixels_to_display_points() {
        let words = parse_tsv(
            &format!("{HEADER}5\t1\t1\t1\t1\t1\t100\t200\t300\t80\t96.1\tété\n"),
            1200,
            1600,
            size(),
        )
        .unwrap();
        assert_eq!(
            words,
            [Word {
                text: "été".into(),
                rect: PointRect {
                    left: 50.0,
                    right: 200.0,
                    bottom: 660.0,
                    top: 700.0
                }
            }]
        );
    }

    #[test]
    fn malformed_or_out_of_bounds_ocr_is_rejected() {
        for (x, y, w, h, confidence, text) in [
            ("NaN", "2", "3", "4", "90", "word"),
            ("1199", "2", "3", "4", "90", "word"),
            ("1", "1599", "3", "4", "90", "word"),
            ("1", "2", "0", "4", "90", "word"),
            ("1", "2", "3", "4", "NaN", "word"),
            ("1", "2", "3", "4", "90", "bad\tword"),
        ] {
            assert!(parse_tsv(
                &format!("{HEADER}5\t1\t1\t1\t1\t1\t{x}\t{y}\t{w}\t{h}\t{confidence}\t{text}\n"),
                1200,
                1600,
                size()
            )
            .is_err());
        }
        assert!(parse_tsv("unexpected output", 1200, 1600, size()).is_err());
    }

    #[test]
    fn render_budget_limits_large_scans() {
        for dimensions in [(612.0, 792.0), (2000.0, 2000.0), (8000.0, 1000.0)] {
            let s = PageSize {
                width_pt: dimensions.0,
                height_pt: dimensions.1,
            };
            let scale = render_scale(s).unwrap();
            assert!(s.width_pt * s.height_pt * scale * scale <= MAX_IMAGE_PIXELS + 1.0);
            assert!(s.width_pt * scale <= 8192.0 && s.height_pt * scale <= 8192.0);
        }
        for dimensions in [(f32::NAN, 20.0), (0.0, 20.0), (20_000.0, 20_000.0)] {
            assert!(render_scale(PageSize {
                width_pt: dimensions.0,
                height_pt: dimensions.1
            })
            .is_err());
        }
    }
}
