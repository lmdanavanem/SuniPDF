//! Stable-Rust, deterministic mutation smoke over the byte-level entry points of the
//! coverage-guided targets in `fuzz/fuzz_targets` (lopdf loading/inspection/rewriting, signature
//! dictionaries, /ByteRange and CMS envelopes). Fixed seed, fixed iteration count and a time box:
//! this is not coverage-guided fuzzing. A panic is caught, its input saved under `target/` and
//! the test fails; an in-process stack overflow or hang is not caught here (the CLI corpus tests
//! run in bounded child processes for that).

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;
use std::time::{Duration, Instant};
use sunipdf_core::{certificate, convert, inspection, security};

const SEED: u64 = 0x5u64 << 60 | 0x0517_2026;
const ITERATIONS: usize = 10_000;
const TIME_BOX: Duration = Duration::from_secs(90);

/// Same calls as `fuzz/fuzz_targets/pdf_structure.rs` and `cms_signatures.rs`.
fn targets(data: &[u8]) {
    let _ = inspection::inspect(data, None);
    let _ = security::strip_metadata(data, None);
    if let Ok(optimized) = convert::optimize(data, None) {
        let _ = inspection::inspect(&optimized, None);
    }
    let _ = certificate::verify_signatures(data, None);
}

/// xorshift64*: reproducible on every platform.
struct Rng(u64);

impl Rng {
    fn below(&mut self, bound: usize) -> usize {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) % bound.max(1) as u64) as usize
    }
}

const TOKENS: &[&[u8]] = &[
    b"0",
    b"-1",
    b"4294967296",
    b"9999999999",
    b" R ",
    b"<<",
    b">>",
    b"[",
    b"]",
    b"()",
    b"<>",
    b"/ByteRange",
    b"/Contents",
    b"/FT /Sig",
    b"/Kids",
    b"/Parent",
    b"/Count",
    b"/Length",
    b"/Filter /FlateDecode",
    b"/Encrypt",
    b"/Prev",
    b"obj",
    b"endobj",
    b"stream\n",
    b"endstream",
    b"xref",
    b"trailer",
    b"startxref",
    b"%%EOF",
    b"\x30\x84\xff\xff\xff\xff",
];

fn mutate(rng: &mut Rng, seeds: &[Vec<u8>], data: &mut Vec<u8>) {
    for _ in 0..=rng.below(4) {
        let at = rng.below(data.len());
        match rng.below(6) {
            0 if !data.is_empty() => data[at] ^= 1 << rng.below(8),
            1 if !data.is_empty() => {
                let bytes = b"\x00\xff09<>[]/ \n";
                data[at] = bytes[rng.below(bytes.len())];
            }
            2 => {
                let end = (at + 1 + rng.below(64)).min(data.len());
                data.drain(at..end);
            }
            3 => {
                let end = (at + 1 + rng.below(256)).min(data.len());
                let chunk = data[at..end].to_vec();
                data.splice(at..at, chunk);
            }
            4 => {
                let token = TOKENS[rng.below(TOKENS.len())];
                data.splice(at..at, token.iter().copied());
            }
            _ => {
                let other = &seeds[rng.below(seeds.len())];
                let from = rng.below(other.len());
                data.truncate(at);
                data.extend_from_slice(&other[from..]);
            }
        }
    }
}

/// One-page PDF with a signature field whose /ByteRange excludes exactly its /Contents token,
/// holding a minimal detached CMS envelope: mutations start past the signature-shape checks.
fn signature_shaped() -> Vec<u8> {
    const CMS: &[u8] = b"\x30\x23\x06\x09\x2a\x86\x48\x86\xf7\x0d\x01\x07\x02\xa0\x16\x30\x14\x02\x01\x01\x31\x00\x30\x0b\x06\x09\x2a\x86\x48\x86\xf7\x0d\x01\x07\x01\x31\x00";
    let hex: String = CMS.iter().map(|b| format!("{b:02x}")).collect();
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /SigFlags 3 >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [4 0 R] >>".to_owned(),
        "<< /Type /Annot /Subtype /Widget /FT /Sig /T (QA) /Rect [0 0 0 0] /P 3 0 R /V 5 0 R >>"
            .to_owned(),
        format!(
            "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached \
             /ByteRange [0 0000000000 0000000000 0000000000] /Contents <{hex}> >>"
        ),
    ];
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", index + 1).as_bytes());
    }
    let xref = out.len();
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    let first = out.windows(10).position(|w| w == b"/Contents ").unwrap() + 10;
    let second = first + hex.len() + 2;
    let range = format!("{first:010} {second:010} {:010}", out.len() - second);
    let at = out
        .windows(32)
        .position(|w| w == b"0000000000 0000000000 0000000000")
        .unwrap();
    out[at..at + 32].copy_from_slice(range.as_bytes());
    out
}

#[test]
fn byte_level_parsers_survive_deterministic_mutations() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut seeds: Vec<Vec<u8>> = std::fs::read_dir(&fixtures)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "pdf"))
        .map(|path| std::fs::read(path).unwrap())
        .collect();
    seeds.sort();
    let signed = signature_shaped();
    // Positive control: the seed reaches signature parsing (no platform verdict is required).
    assert_eq!(
        certificate::verify_signatures(&signed, None).unwrap().len(),
        1
    );
    seeds.push(signed);

    let mut rng = Rng(SEED);
    let started = Instant::now();
    let mut executed = 0;
    let mut crashes = Vec::new();
    let report = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target");
    while executed < ITERATIONS && started.elapsed() < TIME_BOX {
        let mut data = seeds[rng.below(seeds.len())].clone();
        mutate(&mut rng, &seeds, &mut data);
        if catch_unwind(AssertUnwindSafe(|| targets(&data))).is_err() {
            std::fs::create_dir_all(&report).unwrap();
            let path = report.join(format!("fuzz-smoke-crash-{executed:05}.pdf"));
            std::fs::write(&path, &data).unwrap();
            crashes.push(path);
        }
        executed += 1;
    }
    println!(
        "fuzz smoke: seed {SEED:#x}, {executed} inputs from {} seeds in {} ms",
        seeds.len(),
        started.elapsed().as_millis()
    );
    assert!(executed >= 100, "time box allowed only {executed} inputs");
    assert!(crashes.is_empty(), "panicking inputs saved: {crashes:#?}");
}
