//! Native printer selection and bounded raster printing from the current document state.

use crate::dialog::{self, Spec};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use sunipdf_core::{Document, EngineError, PageSize};
use sunipdf_i18n::{tr, tr_args, Language};

#[derive(Default)]
pub struct PrintState {
    pub busy: bool,
    pub done: usize,
    pub total: usize,
    pub cancel: Arc<AtomicBool>,
}

impl PrintState {
    pub fn begin(&mut self, total: usize) {
        self.busy = true;
        self.done = 0;
        self.total = total;
        self.cancel.store(false, Ordering::Relaxed);
    }

    pub fn show(&mut self, ctx: &egui::Context, lang: Language, dark: bool, has_icons: bool) {
        if !self.busy {
            return;
        }
        let mut cancel = false;
        dialog::show(
            ctx,
            Spec::new("print_progress", tr(lang, "print.title"), dark, has_icons),
            |ui| {
                ui.label(tr_args(
                    lang,
                    "print.progress",
                    &[
                        ("done", &self.done.to_string()),
                        ("total", &self.total.to_string()),
                    ],
                ));
                ui.add(
                    egui::ProgressBar::new(self.done as f32 / self.total.max(1) as f32)
                        .show_percentage(),
                );
                if self.cancel.load(Ordering::Relaxed) {
                    ui.label(tr(lang, "print.cancelling"));
                }
            },
            |ui| {
                cancel = dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                    || ui.input(|i| i.key_pressed(egui::Key::Escape));
            },
        );
        if cancel {
            self.cancel.store(true, Ordering::Relaxed);
        }
    }
}

pub struct PrintJob {
    dc: usize,
    pub pages: Vec<usize>,
    output: Option<Vec<u16>>,
}

#[derive(Debug, Clone, Copy)]
pub struct PrintReport {
    pub pages: usize,
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

fn selected_pages(
    count: usize,
    current: usize,
    selected: &[usize],
    ranges: &[(u32, u32)],
    mode: u32,
) -> Result<Vec<usize>, EngineError> {
    let mut pages = match mode {
        1 => selected.to_vec(),
        2 => {
            let mut pages = Vec::new();
            for &(from, to) in ranges {
                if from == 0 || from > to || to as usize > count {
                    return Err(failure("invalid print page range"));
                }
                pages.extend((from as usize - 1)..to as usize);
            }
            pages
        }
        3 => vec![current],
        _ => (0..count).collect(),
    };
    pages.sort_unstable();
    pages.dedup();
    if pages.is_empty() || pages.iter().any(|&page| page >= count) {
        return Err(failure("no valid print pages selected"));
    }
    Ok(pages)
}

#[derive(Debug)]
struct Layout {
    scale: f32,
    render_width: u32,
    render_height: u32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

fn layout(size: PageSize, printable: (i32, i32), dpi: (i32, i32)) -> Result<Layout, EngineError> {
    if !(size.width_pt.is_finite()
        && size.height_pt.is_finite()
        && size.width_pt > 0.0
        && size.height_pt > 0.0)
        || printable.0 <= 0
        || printable.1 <= 0
        || dpi.0 <= 0
        || dpi.1 <= 0
    {
        return Err(failure("invalid printer or page dimensions"));
    }
    let fit = (printable.0 as f32 / dpi.0 as f32 / size.width_pt)
        .min(printable.1 as f32 / dpi.1 as f32 / size.height_pt);
    let width = (size.width_pt * fit * dpi.0 as f32).round() as i32;
    let height = (size.height_pt * fit * dpi.1 as f32).round() as i32;
    let scale = (300.0f32 / 72.0)
        .min(8192.0 / size.width_pt)
        .min(131_000.0 / size.height_pt);
    let (render_width, render_height) =
        sunipdf_core::tiles::page_pixels(size, scale).ok_or(EngineError::RenderTooLarge)?;
    if width <= 0 || height <= 0 {
        return Err(failure("print page is too small"));
    }
    Ok(Layout {
        scale,
        render_width,
        render_height,
        x: (printable.0 - width) / 2,
        y: (printable.1 - height) / 2,
        width,
        height,
    })
}

#[cfg(windows)]
mod native {
    use super::*;
    use std::cell::Cell;
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::{GlobalFree, HWND, RPC_E_CHANGED_MODE};
    use windows_sys::Win32::Graphics::Gdi::*;
    use windows_sys::Win32::Storage::Xps::*;
    use windows_sys::Win32::System::Com::{
        CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED,
    };
    use windows_sys::Win32::UI::Controls::Dialogs::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::IsWindow;

    thread_local! { static CANCEL: Cell<*const AtomicBool> = const { Cell::new(null()) }; }

    unsafe extern "system" fn abort_proc(_dc: HDC, _error: i32) -> i32 {
        CANCEL.with(|flag| {
            let flag = flag.get();
            // The worker keeps the cancellation flag alive until EndDoc/AbortDoc returns.
            i32::from(flag.is_null() || !unsafe { &*flag }.load(Ordering::Relaxed))
        })
    }

    struct AbortFlag;
    impl Drop for AbortFlag {
        fn drop(&mut self) {
            CANCEL.with(|flag| flag.set(null()));
        }
    }

    struct Com(bool);
    impl Drop for Com {
        fn drop(&mut self) {
            if self.0 {
                unsafe {
                    CoUninitialize();
                }
            }
        }
    }

    struct DialogResources(PRINTDLGEXW);
    impl Drop for DialogResources {
        fn drop(&mut self) {
            unsafe {
                if !self.0.hDC.is_null() {
                    DeleteDC(self.0.hDC);
                }
                if !self.0.hDevMode.is_null() {
                    GlobalFree(self.0.hDevMode);
                }
                if !self.0.hDevNames.is_null() {
                    GlobalFree(self.0.hDevNames);
                }
            }
        }
    }

    pub fn choose(
        owner: usize,
        page_count: usize,
        current_page: usize,
        selected: &[usize],
    ) -> Result<Option<PrintJob>, EngineError> {
        if page_count == 0
            || page_count > u32::MAX as usize
            || current_page >= page_count
            || selected.iter().any(|&page| page >= page_count)
        {
            return Err(failure("invalid print page selection"));
        }
        if unsafe { IsWindow(owner as HWND) } == 0 {
            return Err(failure(
                "print dialog requires an active application window",
            ));
        }
        let hr = unsafe { CoInitializeEx(null(), COINIT_APARTMENTTHREADED as u32) };
        if hr < 0 && hr != RPC_E_CHANGED_MODE {
            return Err(failure("cannot initialize the Windows print dialog"));
        }
        let _com = Com(hr >= 0);
        let mut ranges = [PRINTPAGERANGE::default(); 64];
        let mut resources = DialogResources(PRINTDLGEXW {
            lStructSize: std::mem::size_of::<PRINTDLGEXW>() as u32,
            hwndOwner: owner as HWND,
            Flags: PD_RETURNDC
                | PD_USEDEVMODECOPIESANDCOLLATE
                | if selected.is_empty() {
                    PD_NOSELECTION
                } else {
                    0
                },
            nMaxPageRanges: ranges.len() as u32,
            lpPageRanges: ranges.as_mut_ptr(),
            nMinPage: 1,
            nMaxPage: page_count as u32,
            nCopies: 1,
            nStartPage: START_PAGE_GENERAL,
            ..Default::default()
        });
        let hr = unsafe { PrintDlgExW(&mut resources.0) };
        if hr < 0 {
            return Err(failure(&format!(
                "Windows print dialog failed (0x{:08X})",
                hr as u32
            )));
        }
        let pd = &mut resources.0;
        if pd.dwResultAction != PD_RESULT_PRINT {
            return Ok(None);
        }
        if pd.hDC.is_null() || pd.nPageRanges as usize > ranges.len() {
            return Err(failure("Windows returned an invalid printer"));
        }
        let mode = if pd.Flags & PD_SELECTION != 0 {
            1
        } else if pd.Flags & PD_PAGENUMS != 0 {
            2
        } else if pd.Flags & PD_CURRENTPAGE != 0 {
            3
        } else {
            0
        };
        let ranges: Vec<_> = ranges[..pd.nPageRanges as usize]
            .iter()
            .map(|range| (range.nFromPage, range.nToPage))
            .collect();
        let pages = selected_pages(page_count, current_page, selected, &ranges, mode)?;
        let output = (pd.Flags & PD_PRINTTOFILE != 0).then(|| "FILE:\0".encode_utf16().collect());
        let job = PrintJob {
            dc: pd.hDC as usize,
            pages,
            output,
        };
        pd.hDC = null_mut();
        Ok(Some(job))
    }

    struct ActiveDoc {
        dc: HDC,
        finished: bool,
    }
    impl Drop for ActiveDoc {
        fn drop(&mut self) {
            if !self.finished {
                unsafe {
                    AbortDoc(self.dc);
                }
            }
        }
    }

    fn gdi_result(result: i32, operation: &str, cancel: &AtomicBool) -> Result<(), EngineError> {
        check_cancel(cancel)?;
        if result <= 0 {
            Err(failure(&format!(
                "Windows printing failed during {operation}"
            )))
        } else {
            Ok(())
        }
    }

    pub fn print(
        doc: &Document<'_>,
        job: PrintJob,
        title: &str,
        cancel: &AtomicBool,
        mut progress: impl FnMut(usize, usize),
    ) -> Result<PrintReport, EngineError> {
        check_cancel(cancel)?;
        if !doc.properties().can_print {
            return Err(EngineError::SecurityRestricted);
        }
        if job.dc == 0 || job.pages.is_empty() {
            return Err(failure("invalid print job"));
        }
        for &page in &job.pages {
            doc.page_size(page)?;
        }
        let dc = job.dc as HDC;
        let printable = unsafe {
            (
                GetDeviceCaps(dc, HORZRES as i32),
                GetDeviceCaps(dc, VERTRES as i32),
            )
        };
        let dpi = unsafe {
            (
                GetDeviceCaps(dc, LOGPIXELSX as i32),
                GetDeviceCaps(dc, LOGPIXELSY as i32),
            )
        };
        // Validate every page before starting the spool job.
        let layouts = job
            .pages
            .iter()
            .map(|&page| layout(doc.page_size(page)?, printable, dpi))
            .collect::<Result<Vec<_>, EngineError>>()?;
        CANCEL.with(|flag| flag.set(cancel));
        let _flag = AbortFlag;
        gdi_result(
            unsafe { SetAbortProc(dc, Some(abort_proc)) },
            "cancellation setup",
            cancel,
        )?;
        let title: Vec<u16> = title
            .chars()
            .filter(|&c| c != '\0')
            .take(240)
            .collect::<String>()
            .encode_utf16()
            .chain([0])
            .collect();
        let info = DOCINFOW {
            cbSize: std::mem::size_of::<DOCINFOW>() as i32,
            lpszDocName: title.as_ptr(),
            lpszOutput: job.output.as_ref().map_or(null(), |path| path.as_ptr()),
            ..Default::default()
        };
        gdi_result(unsafe { StartDocW(dc, &info) }, "starting the job", cancel)?;
        let mut active = ActiveDoc {
            dc,
            finished: false,
        };
        unsafe {
            SetStretchBltMode(dc, HALFTONE);
            SetBrushOrgEx(dc, 0, 0, null_mut());
        }
        for (done, (&page, layout)) in job.pages.iter().zip(layouts.iter()).enumerate() {
            check_cancel(cancel)?;
            gdi_result(unsafe { StartPage(dc) }, "starting a page", cancel)?;
            for y in (0..layout.render_height).step_by(256) {
                check_cancel(cancel)?;
                let height = 256.min(layout.render_height - y);
                let mut image = doc.render_region(
                    page,
                    layout.scale,
                    sunipdf_core::PixelRect::new(0, y, layout.render_width, height),
                )?;
                for pixel in image.rgba.as_chunks_mut::<4>().0 {
                    let alpha = u16::from(pixel[3]);
                    for channel in pixel[..3].iter_mut() {
                        *channel =
                            ((u16::from(*channel) * alpha + 255 * (255 - alpha) + 127) / 255) as u8;
                    }
                    pixel.swap(0, 2);
                    pixel[3] = 0;
                }
                let bitmap = BITMAPINFO {
                    bmiHeader: BITMAPINFOHEADER {
                        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                        biWidth: image.width as i32,
                        biHeight: -(image.height as i32),
                        biPlanes: 1,
                        biBitCount: 32,
                        biCompression: BI_RGB,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                let start =
                    (u64::from(y) * layout.height as u64 / u64::from(layout.render_height)) as i32;
                let end = (u64::from(y + height) * layout.height as u64
                    / u64::from(layout.render_height)) as i32;
                if end == start {
                    continue;
                }
                let result = unsafe {
                    StretchDIBits(
                        dc,
                        layout.x,
                        layout.y + start,
                        layout.width,
                        end - start,
                        0,
                        0,
                        image.width as i32,
                        image.height as i32,
                        image.rgba.as_ptr().cast(),
                        &bitmap,
                        DIB_RGB_COLORS,
                        SRCCOPY,
                    )
                };
                if result == GDI_ERROR || result == 0 {
                    check_cancel(cancel)?;
                    return Err(failure("Windows could not print the page raster"));
                }
            }
            gdi_result(unsafe { EndPage(dc) }, "finishing a page", cancel)?;
            progress(done + 1, job.pages.len());
        }
        check_cancel(cancel)?;
        gdi_result(unsafe { EndDoc(dc) }, "finishing the job", cancel)?;
        active.finished = true;
        Ok(PrintReport {
            pages: job.pages.len(),
        })
    }
}

impl Drop for PrintJob {
    fn drop(&mut self) {
        #[cfg(windows)]
        if self.dc != 0 {
            unsafe {
                windows_sys::Win32::Graphics::Gdi::DeleteDC(self.dc as _);
            }
        }
    }
}

#[cfg(windows)]
pub use native::{choose, print};

#[cfg(not(windows))]
pub fn choose(
    _owner: usize,
    _page_count: usize,
    _current_page: usize,
    _selected: &[usize],
) -> Result<Option<PrintJob>, EngineError> {
    Err(failure("native printing requires Windows"))
}

#[cfg(not(windows))]
pub fn print(
    _doc: &Document<'_>,
    _job: PrintJob,
    _title: &str,
    _cancel: &AtomicBool,
    _progress: impl FnMut(usize, usize),
) -> Result<PrintReport, EngineError> {
    Err(failure("native printing requires Windows"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_selection_and_current_page_are_validated() {
        assert_eq!(
            selected_pages(6, 2, &[], &[(1, 2), (2, 4), (6, 6)], 2).unwrap(),
            [0, 1, 2, 3, 5]
        );
        assert_eq!(selected_pages(6, 2, &[], &[], 3).unwrap(), [2]);
        assert_eq!(selected_pages(6, 2, &[5, 0, 5], &[], 1).unwrap(), [0, 5]);
        for ranges in [&[(0, 1)][..], &[(4, 3)][..], &[(1, 7)][..], &[][..]] {
            assert!(selected_pages(6, 0, &[], ranges, 2).is_err());
        }
        assert!(selected_pages(6, 6, &[], &[], 3).is_err());
        assert!(selected_pages(6, 0, &[6], &[], 1).is_err());
    }

    #[test]
    fn layout_fits_printable_area_with_non_square_dpi_and_bounded_strips() {
        let size = PageSize {
            width_pt: 612.0,
            height_pt: 792.0,
        };
        let l = layout(size, (4800, 6600), (600, 600)).unwrap();
        assert!(l.x >= 0 && l.y >= 0 && l.width <= 4800 && l.height <= 6600);
        assert!((l.width as f32 / l.height as f32 - size.width_pt / size.height_pt).abs() < 0.001);
        let l = layout(size, (4800, 3300), (600, 300)).unwrap();
        assert!(
            (l.width as f32 / 600.0 / (l.height as f32 / 300.0) - size.width_pt / size.height_pt)
                .abs()
                < 0.001
        );
        let l = layout(
            PageSize {
                width_pt: 40_000.0,
                height_pt: 100_000.0,
            },
            (4800, 6600),
            (600, 600),
        )
        .unwrap();
        assert!(l.render_width <= 8192 && l.render_height <= 131_072);
        assert!(layout(size, (0, 6600), (600, 600)).is_err());
        assert!(layout(
            PageSize {
                width_pt: f32::NAN,
                height_pt: 100.0
            },
            (4800, 6600),
            (600, 600)
        )
        .is_err());
    }

    #[test]
    fn cancellation_is_typed() {
        assert!(matches!(
            check_cancel(&AtomicBool::new(true)),
            Err(EngineError::Cancelled)
        ));
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requires the Windows Microsoft Print to PDF driver; writes only a temporary PDF"]
    fn prints_current_edits_to_microsoft_pdf() {
        use windows_sys::Win32::Graphics::Gdi::CreateDCW;
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("printed.pdf");
        let printer: Vec<u16> = "Microsoft Print to PDF\0".encode_utf16().collect();
        let driver: Vec<u16> = "WINSPOOL\0".encode_utf16().collect();
        let dc = unsafe {
            CreateDCW(
                driver.as_ptr(),
                printer.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
            )
        };
        assert!(!dc.is_null(), "Microsoft Print to PDF must be installed");
        let engine = Box::leak(Box::new(sunipdf_core::Engine::new().unwrap()));
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf");
        let original = std::fs::read(&fixture).unwrap();
        let mut doc = engine.open(&fixture, None).unwrap();
        let indices: Vec<_> = doc
            .page_objects(0)
            .unwrap()
            .iter()
            .map(|object| object.index)
            .collect();
        if !indices.is_empty() {
            doc.delete_objects(0, &indices).unwrap();
        }
        assert!(doc
            .render_page(0, 0.5)
            .unwrap()
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| pixel[0] > 250 && pixel[1] > 250 && pixel[2] > 250));
        doc.add_text(
            0,
            40.0,
            500.0,
            "UNSAVED PRINT TEST",
            &sunipdf_core::TextStyle {
                size_pt: 28.0,
                ..Default::default()
            },
        )
        .unwrap();
        let before = doc.snapshot().unwrap();
        assert!(matches!(
            print(
                &doc,
                PrintJob {
                    dc: 0,
                    pages: vec![0],
                    output: None
                },
                "Cancelled verification",
                &AtomicBool::new(true),
                |_, _| {}
            ),
            Err(EngineError::Cancelled)
        ));
        use std::os::windows::ffi::OsStrExt;
        let job = PrintJob {
            dc: dc as usize,
            pages: vec![0],
            output: Some(output.as_os_str().encode_wide().chain([0]).collect()),
        };
        let mut updates = Vec::new();
        let report = print(
            &doc,
            job,
            "SuniPDF print verification",
            &AtomicBool::new(false),
            |done, total| updates.push((done, total)),
        )
        .unwrap();
        assert_eq!(report.pages, 1);
        assert_eq!(updates, [(1, 1)]);
        let mut before = lopdf::Document::load_mem(&before).unwrap();
        let mut after = lopdf::Document::load_mem(&doc.snapshot().unwrap()).unwrap();
        // PDFium can regenerate only the trailer ID when serializing an unchanged document.
        before.trailer.remove(b"ID");
        after.trailer.remove(b"ID");
        assert_eq!(after.objects, before.objects);
        assert_eq!(after.trailer, before.trailer);
        assert_eq!(std::fs::read(&fixture).unwrap(), original);
        let started = std::time::Instant::now();
        loop {
            if let Ok(bytes) = std::fs::read(&output) {
                if bytes.starts_with(b"%PDF")
                    && bytes
                        .windows(5)
                        .rev()
                        .take(1024)
                        .any(|part| part == b"%%EOF")
                {
                    break;
                }
            }
            assert!(
                started.elapsed() < std::time::Duration::from_secs(20),
                "Microsoft Print to PDF did not finish its output"
            );
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let printed = engine.open(&output, None).unwrap();
        assert_eq!(printed.page_count(), 1);
        let pixels = printed.render_page(0, 0.5).unwrap();
        assert!(pixels
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[0] < 100 && pixel[1] < 100 && pixel[2] < 100));
    }
}
