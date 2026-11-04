//! Hostile and large PDFs generated deterministically at test time (nothing is committed).
//! Every command runs in a child process under time, memory and output guards; each outcome is
//! either a bounded success or a typed JSON error (`EngineError::code()`). Each case pins its
//! `info` outcome; published outputs must reopen and sources must stay byte-identical.
#![cfg(windows)]

#[path = "support/bounded_process.rs"]
mod process;

use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(60);
const BUDGET: Duration = Duration::from_secs(900);

/// Assembles numbered objects (`objects[i]` is object `i + 1`) with a correct xref table.
fn pdf(objects: &[Vec<u8>], trailer: &str) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    let trailer = trailer.replace("{xref}", &xref.to_string());
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R {trailer} >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

fn obj(text: impl Into<String>) -> Vec<u8> {
    text.into().into_bytes()
}

fn stream(dict: &str, content: &[u8]) -> Vec<u8> {
    let mut out = format!("<< {dict} /Length {} >>\nstream\n", content.len()).into_bytes();
    out.extend_from_slice(content);
    out.extend_from_slice(b"\nendstream");
    out
}

fn flate(content: Vec<u8>) -> Vec<u8> {
    let mut stream = lopdf::Stream::new(lopdf::Dictionary::new(), content);
    stream.compress().unwrap();
    assert!(
        stream.dict.get(b"Filter").is_ok(),
        "generated stream must compress"
    );
    stream.content
}

const FONT: &str = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>";

/// Catalog (1), Pages (2), one page (3) with `page_extra`, content (4), font (5), then `extra`.
fn one_page(page_extra: &str, content: &[u8], extra: Vec<Vec<u8>>) -> Vec<u8> {
    let mut objects = vec![
        obj("<< /Type /Catalog /Pages 2 0 R >>"),
        obj("<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        obj(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
             /Resources << /Font << /F1 5 0 R >> {page_extra} >> >>"
        )),
        stream("", content),
        obj(FONT),
    ];
    objects.extend(extra);
    pdf(&objects, "")
}

const HELLO: &[u8] = b"BT /F1 24 Tf 72 700 Td (Hostile corpus) Tj ET";

/// Expected `info` outcome: page count on success, or the stable `EngineError::code()`.
enum Open {
    Pages(u64),
    Error(&'static str),
}

struct Case {
    name: &'static str,
    bytes: Vec<u8>,
    open: Open,
    /// `render` error expected even though the document opens (e.g. oversized page).
    render_error: Option<&'static str>,
    /// Known defect (docs/KNOWN_ISSUES.md): `optimize` publishes a copy that does not reopen with
    /// the source's page count. Pinned so a fix or a new occurrence both fail this test.
    optimize_unreopenable: bool,
}

fn case(name: &'static str, bytes: Vec<u8>, open: Open) -> Case {
    Case {
        name,
        bytes,
        open,
        render_error: None,
        optimize_unreopenable: false,
    }
}

fn unreopenable_optimize(case: Case) -> Case {
    Case {
        optimize_unreopenable: true,
        ..case
    }
}

fn hostile_cases() -> Vec<Case> {
    let valid = one_page("", HELLO, vec![]);
    let mut cases = vec![
        case("empty-file", Vec::new(), Open::Error("invalid_format")),
        case(
            "html-not-pdf",
            b"<!doctype html><title>x</title><script>alert(1)</script>".to_vec(),
            Open::Error("invalid_format"),
        ),
        case(
            "header-only",
            b"%PDF-1.7\n%%EOF\n".to_vec(),
            Open::Error("invalid_format"),
        ),
        unreopenable_optimize(case(
            "missing-root",
            {
                let mut bytes = valid.clone();
                let at = bytes.windows(5).rposition(|w| w == b"/Root").unwrap();
                bytes[at..at + 5].copy_from_slice(b"/Xxxx");
                bytes
            },
            Open::Error("invalid_format"),
        )),
        case(
            "xref-offsets-zeroed",
            {
                let mut bytes = valid.clone();
                let xref = bytes.windows(5).rposition(|w| w == b"xref\n").unwrap();
                for byte in &mut bytes[xref..] {
                    if (b'1'..=b'9').contains(byte) {
                        *byte = b'0';
                    }
                }
                bytes
            },
            Open::Pages(1),
        ),
        case(
            "xref-prev-self-loop",
            one_page_trailer("/Prev {xref}"),
            Open::Pages(1),
        ),
        unreopenable_optimize(case(
            "pages-kids-self-cycle",
            pdf(
                &[
                    obj("<< /Type /Catalog /Pages 2 0 R >>"),
                    obj("<< /Type /Pages /Kids [2 0 R] /Count 1 >>"),
                ],
                "",
            ),
            Open::Error("other"),
        )),
        unreopenable_optimize(case(
            "pages-mutual-cycle",
            pdf(
                &[
                    obj("<< /Type /Catalog /Pages 2 0 R >>"),
                    obj("<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>"),
                    obj("<< /Type /Pages /Parent 2 0 R /Kids [2 0 R] /Count 1 >>"),
                    obj("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>"),
                ],
                "",
            ),
            Open::Error("other"),
        )),
        case(
            "pages-count-overflow",
            pdf(
                &[
                    obj("<< /Type /Catalog /Pages 2 0 R >>"),
                    obj("<< /Type /Pages /Kids [3 0 R] /Count 2147483647 >>"),
                    obj("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>"),
                ],
                "",
            ),
            Open::Pages(1),
        ),
        unreopenable_optimize(case(
            "pages-tree-deep-4000",
            deep_pages_tree(4000),
            Open::Error("other"),
        )),
        case(
            "length-reference-cycle",
            pdf(
                &[
                    obj("<< /Type /Catalog /Pages 2 0 R >>"),
                    obj("<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
                    obj("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>"),
                    {
                        let mut body = b"<< /Length 5 0 R >>\nstream\n".to_vec();
                        body.extend_from_slice(b"0 0 m 100 100 l S");
                        body.extend_from_slice(b"\nendstream");
                        body
                    },
                    obj("6 0 R"),
                    obj("5 0 R"),
                ],
                "",
            ),
            Open::Pages(1),
        ),
        case(
            "form-xobject-self-recursion",
            one_page(
                "/XObject << /Fm0 6 0 R >>",
                b"/Fm0 Do",
                vec![stream(
                    "/Type /XObject /Subtype /Form /BBox [0 0 612 792] \
                     /Resources << /XObject << /Fm0 6 0 R >> >>",
                    b"0 0 m 10 10 l S /Fm0 Do",
                )],
            ),
            Open::Pages(1),
        ),
        case(
            "form-xobject-mutual-recursion",
            one_page(
                "/XObject << /Fm0 6 0 R >>",
                b"/Fm0 Do",
                vec![
                    stream(
                        "/Type /XObject /Subtype /Form /BBox [0 0 612 792] \
                         /Resources << /XObject << /Fm1 7 0 R >> >>",
                        b"/Fm1 Do",
                    ),
                    stream(
                        "/Type /XObject /Subtype /Form /BBox [0 0 612 792] \
                         /Resources << /XObject << /Fm0 6 0 R >> >>",
                        b"/Fm0 Do",
                    ),
                ],
            ),
            Open::Pages(1),
        ),
        case("form-xobject-nested-64", nested_xobjects(64), Open::Pages(1)),
        case(
            "content-array-nesting-100k",
            one_page("", &[b'['; 100_000], vec![]),
            Open::Pages(1),
        ),
        case(
            "object-dictionary-nesting-20k",
            one_page(
                "",
                HELLO,
                vec![obj(format!("{}{}", "<< /A ".repeat(20_000), ">> ".repeat(20_000)))],
            ),
            Open::Pages(1),
        ),
        case(
            "flate-bomb-128mib",
            pdf(
                &[
                    obj("<< /Type /Catalog /Pages 2 0 R >>"),
                    obj("<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
                    obj("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>"),
                    stream("/Filter /FlateDecode", &flate(vec![b' '; 128 << 20])),
                ],
                "",
            ),
            Open::Pages(1),
        ),
        case(
            "javascript-actions",
            pdf(
                &[
                    obj("<< /Type /Catalog /Pages 2 0 R /OpenAction 6 0 R \
                         /Names << /JavaScript << /Names [(boot) 6 0 R] >> >> /AcroForm << /Fields [] /NeedsRendering true >> >>"),
                    obj("<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
                    obj("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
                         /Resources << /Font << /F1 5 0 R >> >> /AA << /O 6 0 R /C 6 0 R >> >>"),
                    stream("", HELLO),
                    obj(FONT),
                    obj("<< /S /JavaScript /JS (app.launchURL\\('https://example.invalid/', true\\); \
                         while\\(true\\){}) /Next << /S /Launch /F (cmd.exe) >> >>"),
                ],
                "",
            ),
            Open::Pages(1),
        ),
        case(
            "type3-font-recursive-glyph",
            pdf(
                &[
                    obj("<< /Type /Catalog /Pages 2 0 R >>"),
                    obj("<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
                    obj("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
                         /Resources << /Font << /T3 5 0 R >> >> >>"),
                    stream("", b"BT /T3 48 Tf 72 600 Td (aaaa) Tj ET"),
                    obj("<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1000 1000] \
                         /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs << /a 6 0 R >> \
                         /Encoding << /Type /Encoding /Differences [97 /a] >> /FirstChar 97 \
                         /LastChar 97 /Widths [1000] /Resources << /Font << /T3 5 0 R >> >> >>"),
                    stream("", b"1000 0 0 0 1000 1000 d1 0 0 500 500 re f BT /T3 1 Tf (a) Tj ET"),
                ],
                "",
            ),
            Open::Pages(1),
        ),
        case(
            "image-declared-huge-truncated-data",
            one_page(
                "/XObject << /Im0 6 0 R >>",
                b"q 612 0 0 792 0 0 cm /Im0 Do Q",
                vec![stream(
                    "/Type /XObject /Subtype /Image /Width 100000 /Height 100000 \
                     /ColorSpace /DeviceRGB /BitsPerComponent 8",
                    &[0x7f; 64],
                )],
            ),
            Open::Pages(1),
        ),
        unreopenable_optimize(case(
            "mediabox-degenerate",
            pdf(
                &[
                    obj("<< /Type /Catalog /Pages 2 0 R >>"),
                    obj("<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>"),
                    obj("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 0 0] >>"),
                    obj("<< /Type /Page /Parent 2 0 R /MediaBox [100 100 -1e30 1e30] >>"),
                ],
                "",
            ),
            Open::Pages(2),
        )),
        case(
            "encrypt-unknown-filter",
            one_page_trailer_with(
                "/Encrypt 6 0 R /ID [<00112233445566778899AABBCCDDEEFF> <00112233445566778899AABBCCDDEEFF>]",
                vec![obj("<< /Filter /Bogus /V 9 /R 9 /Length 512 >>")],
            ),
            Open::Error("security_restricted"),
        ),
    ];
    cases.push(Case {
        render_error: Some("render_too_large"),
        ..case(
            // 20000 pt at scale 1 exceeds the 16384 px render side limit.
            "page-mediabox-oversized",
            pdf(
                &[
                    obj("<< /Type /Catalog /Pages 2 0 R >>"),
                    obj("<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
                    obj("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20000 20000] >>"),
                ],
                "",
            ),
            Open::Pages(1),
        )
    });
    cases
}

fn one_page_trailer(trailer: &str) -> Vec<u8> {
    one_page_trailer_with(trailer, vec![])
}

fn one_page_trailer_with(trailer: &str, extra: Vec<Vec<u8>>) -> Vec<u8> {
    let mut objects = vec![
        obj("<< /Type /Catalog /Pages 2 0 R >>"),
        obj("<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        obj(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
             /Resources << /Font << /F1 5 0 R >> >> >>",
        ),
        stream("", HELLO),
        obj(FONT),
    ];
    objects.extend(extra);
    pdf(&objects, trailer)
}

/// A chain of single-child intermediate Pages nodes, `depth` deep, ending in one page.
fn deep_pages_tree(depth: usize) -> Vec<u8> {
    let mut objects = vec![obj("<< /Type /Catalog /Pages 2 0 R >>")];
    for level in 0..depth {
        let number = level + 2;
        let parent = if level == 0 {
            String::new()
        } else {
            format!("/Parent {} 0 R", number - 1)
        };
        objects.push(obj(format!(
            "<< /Type /Pages {parent} /Kids [{} 0 R] /Count 1 >>",
            number + 1
        )));
    }
    objects.push(obj(format!(
        "<< /Type /Page /Parent {} 0 R /MediaBox [0 0 612 792] >>",
        depth + 1
    )));
    pdf(&objects, "")
}

/// Legitimate (acyclic) Form XObjects nested `depth` levels, each drawing a rectangle.
fn nested_xobjects(depth: usize) -> Vec<u8> {
    let mut extra = Vec::new();
    for level in 0..depth {
        let number = level + 6;
        let child = if level + 1 < depth {
            format!("/Resources << /XObject << /X {} 0 R >> >>", number + 1)
        } else {
            String::new()
        };
        let draw = if level + 1 < depth { "/X Do" } else { "" };
        extra.push(stream(
            &format!("/Type /XObject /Subtype /Form /BBox [0 0 612 792] {child}"),
            format!("q 0.98 0 0 0.98 6 8 cm 0 0 600 780 re S {draw} Q").as_bytes(),
        ));
    }
    one_page("/XObject << /X 6 0 R >>", b"/X Do", extra)
}

/// Large inputs generated at test time: many pages, one A4 600 dpi scan, one dense content stream.
fn large_cases() -> Vec<Case> {
    const PAGES: usize = 3000;
    let mut objects = vec![obj("<< /Type /Catalog /Pages 2 0 R >>")];
    let kids: Vec<String> = (0..PAGES).map(|i| format!("{} 0 R", 4 + 2 * i)).collect();
    objects.push(obj(format!(
        "<< /Type /Pages /Kids [{}] /Count {PAGES} >>",
        kids.join(" ")
    )));
    objects.push(obj(FONT));
    for index in 0..PAGES {
        objects.push(obj(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents {} 0 R \
             /Resources << /Font << /F1 3 0 R >> >> >>",
            5 + 2 * index
        )));
        objects.push(stream(
            "",
            format!("BT /F1 18 Tf 72 700 Td (Large page {}) Tj ET", index + 1).as_bytes(),
        ));
    }
    let many_pages = pdf(&objects, "");

    // 4960 x 7016 px: A4 at 600 dpi, a deterministic grey pattern encoded as baseline JPEG.
    let (width, height) = (4960u32, 7016u32);
    let scan = image::GrayImage::from_fn(width, height, |x, y| {
        image::Luma([if (x / 40 + y / 40) % 2 == 0 { 235 } else { 30 }])
    });
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 75)
        .encode_image(&scan)
        .unwrap();
    let large_scan = one_page(
        "/XObject << /Im0 6 0 R >>",
        b"q 595 0 0 842 0 0 cm /Im0 Do Q BT /F1 12 Tf 40 20 Td (Large scan) Tj ET",
        vec![stream(
            &format!(
                "/Type /XObject /Subtype /Image /Width {width} /Height {height} \
                 /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /DCTDecode"
            ),
            &jpeg,
        )],
    );

    let mut dense = Vec::new();
    for index in 0..100_000u32 {
        let (x, y) = (index % 600, (index / 600) % 780);
        dense.extend_from_slice(format!("{x} {y} 1 1 re ").as_bytes());
        if index % 1000 == 999 {
            dense.extend_from_slice(b"f\n");
        }
    }
    dense.extend_from_slice(b"f ");
    dense.extend_from_slice(HELLO);
    let dense_stream = pdf(
        &[
            obj("<< /Type /Catalog /Pages 2 0 R >>"),
            obj("<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
            obj(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
                 /Resources << /Font << /F1 5 0 R >> >> >>",
            ),
            stream("/Filter /FlateDecode", &flate(dense)),
            obj(FONT),
        ],
        "",
    );

    vec![
        case("large-3000-pages", many_pages, Open::Pages(PAGES as u64)),
        case("large-scan-a4-600dpi", large_scan, Open::Pages(1)),
        case("large-dense-content-100k-ops", dense_stream, Open::Pages(1)),
    ]
}

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"))
}

/// Runs one command; `Ok(stdout)` on success, `Err(code)` for a typed JSON error.
fn run(command: Command) -> (Result<Vec<u8>, String>, Option<i32>) {
    let output = process::bounded_for(command, TIMEOUT);
    let code = output.status.code().expect("native process crashed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("panicked"), "panic output: {stderr}");
    if code == 0 {
        return (Ok(output.stdout), Some(code));
    }
    assert!(
        matches!(code, 1 | 3 | 4 | 5),
        "unexpected exit {code}: {stderr}"
    );
    let report: Value =
        serde_json::from_slice(&output.stderr).expect("typed JSON error, never panic output");
    (
        Err(report["error"].as_str().expect("error code").to_owned()),
        Some(code),
    )
}

fn reopened_pages(path: &Path) -> Value {
    let mut info = cli();
    info.arg("info").arg(path);
    match run(info).0 {
        Ok(stdout) => serde_json::from_slice::<Value>(&stdout).unwrap()["page_count"].clone(),
        Err(code) => json!({ "reopen_error": code }),
    }
}

fn exercise(temp: &Path, case: &Case) -> Value {
    let source = temp.join(format!("{}.pdf", case.name));
    std::fs::write(&source, &case.bytes).unwrap();
    let mut record = json!({"case": case.name, "bytes": case.bytes.len()});

    let mut info = cli();
    info.arg("info").arg(&source);
    let (opened, exit) = run(info);
    record["info_exit"] = json!(exit);
    // Recorded rather than asserted at once so one run lists every drifted outcome.
    let actual = match &opened {
        Ok(stdout) => {
            let report: Value = serde_json::from_slice(stdout).unwrap();
            json!({"page_count": report["page_count"]})
        }
        Err(code) => json!({"error": code}),
    };
    let expected = match case.open {
        Open::Pages(count) => json!({"page_count": count}),
        Open::Error(code) => json!({"error": code}),
    };
    record["open"] = actual.clone();
    if actual != expected {
        record["open_mismatch"] = expected;
    }

    let mut text = cli();
    text.arg("text").arg(&source);
    let (extracted, exit) = run(text);
    record["text_exit"] = json!(exit);
    if let Err(code) = &extracted {
        record["text_error"] = json!(code);
    }

    let pages = match &case.open {
        Open::Pages(count) => *count,
        Open::Error(_) => 1,
    };
    let rendered_page = temp.join(format!("{}.png", case.name));
    let mut render = cli();
    render
        .arg("render")
        .arg(&source)
        .args(["--page", &pages.max(1).to_string(), "--scale", "1"])
        .arg("--out")
        .arg(&rendered_page);
    let (render_result, exit) = run(render);
    record["render_exit"] = json!(exit);
    match (&render_result, case.render_error) {
        (Ok(_), None) => assert!(rendered_page.metadata().unwrap().len() > 0, "{}", case.name),
        (Err(code), expected) => {
            assert!(
                !rendered_page.exists(),
                "{}: failed render wrote",
                case.name
            );
            record["render_error"] = json!(code);
            if expected.is_some_and(|expected| expected != code) {
                record["render_mismatch"] = json!(expected);
            }
        }
        (Ok(_), Some(expected)) => record["render_mismatch"] = json!(expected),
    }

    let optimized = temp.join(format!("{}-optimized.pdf", case.name));
    let mut optimize = cli();
    optimize
        .arg("optimize")
        .arg(&source)
        .arg("--out")
        .arg(&optimized);
    let (saved, exit) = run(optimize);
    record["optimize_exit"] = json!(exit);
    let mut unreopenable = false;
    match saved {
        Ok(_) => {
            record["optimize_page_count"] = reopened_pages(&optimized);
            // A published copy must reopen with the page count PDFium reads from the source.
            unreopenable = record["open"]["page_count"] != record["optimize_page_count"];
            record["optimize_unreopenable"] = json!(unreopenable);
        }
        Err(code) => {
            assert!(
                !optimized.exists(),
                "{}: failed save published output",
                case.name
            );
            record["optimize_error"] = json!(code);
        }
    }
    if unreopenable != case.optimize_unreopenable {
        record["optimize_page_mismatch"] = json!(true);
    }

    let redacted = temp.join(format!("{}-redacted.pdf", case.name));
    let mut redact = cli();
    redact
        .arg("redact")
        .arg(&source)
        .args(["--rect", "1:72,690,300,730", "--dpi", "72"])
        .arg("--out")
        .arg(&redacted);
    let (redaction, exit) = run(redact);
    record["redact_exit"] = json!(exit);
    match redaction {
        Ok(_) => {
            record["redact_page_count"] = reopened_pages(&redacted);
            if record["open"]["page_count"] != record["redact_page_count"] {
                record["redact_page_mismatch"] = json!(true);
            }
            let mut text = cli();
            text.arg("text").arg(&redacted).args(["--page", "1"]);
            if let Ok(stdout) = run(text).0 {
                let remaining = String::from_utf8_lossy(&stdout);
                assert!(
                    !remaining.contains("Hostile corpus") && !remaining.contains("Large page"),
                    "{}: redacted text still extractable",
                    case.name
                );
            }
        }
        Err(code) => {
            assert!(
                !redacted.exists(),
                "{}: failed redaction published output",
                case.name
            );
            record["redact_error"] = json!(code);
        }
    }

    assert_eq!(
        std::fs::read(&source).unwrap(),
        case.bytes,
        "{}: source modified",
        case.name
    );
    record["source_unchanged"] = json!(true);
    record
}

#[test]
fn hostile_and_large_generated_pdfs_yield_typed_bounded_outcomes() {
    let temp = tempfile::tempdir().unwrap();
    let started = Instant::now();
    let mut records = Vec::new();
    for case in hostile_cases().into_iter().chain(large_cases()) {
        assert!(
            started.elapsed() < BUDGET,
            "corpus exceeded its total budget"
        );
        let case_started = Instant::now();
        let first_command = process::COMMAND_METRICS.lock().unwrap().len();
        let mut record = exercise(temp.path(), &case);
        record["elapsed_ms"] = json!(case_started.elapsed().as_millis());
        record["peak_working_set_bytes"] = process::COMMAND_METRICS.lock().unwrap()
            [first_command..]
            .iter()
            .filter_map(|command| command["peak_working_set_bytes"].as_u64())
            .max()
            .into();
        println!("{record}");
        records.push(record);
    }
    let report = json!({
        "campaign": "generated hostile and large PDF corpus",
        "cases": records,
        "elapsed_ms": started.elapsed().as_millis(),
        "commands": *process::COMMAND_METRICS.lock().unwrap(),
        "hardware": process::hardware(),
        "per_command_timeout_seconds": TIMEOUT.as_secs(),
        "campaign_budget_seconds": BUDGET.as_secs(),
        "memory_guard_bytes": process::MAX_MEMORY,
        "output_limit_bytes": process::MAX_OUTPUT,
        "third_party_documents": false,
    });
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("hostile-corpus-report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    let drifted: Vec<&Value> = report["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|record| {
            [
                "open_mismatch",
                "render_mismatch",
                "optimize_page_mismatch",
                "redact_page_mismatch",
            ]
            .iter()
            .any(|key| record.get(key).is_some())
        })
        .collect();
    assert!(
        drifted.is_empty(),
        "pinned outcomes changed or a saved copy lost pages: {drifted:#?}"
    );
}
