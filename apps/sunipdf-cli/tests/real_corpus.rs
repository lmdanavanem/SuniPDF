//! Pinned, licensed public research PDFs. Downloads and derived copies stay outside Git/packages.
#![cfg(target_os = "windows")]

use std::io::Write;
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const MAX_MEMORY: usize = 768 * 1024 * 1024;
const MAX_OUTPUT: u64 = 2 * 1024 * 1024;
static PEAK_MEMORY: AtomicUsize = AtomicUsize::new(0);

#[repr(C)]
#[derive(Default)]
struct ProcessMemory {
    size: u32,
    page_faults: u32,
    peak_working_set: usize,
    working_set: usize,
    quota_peak_paged: usize,
    quota_paged: usize,
    quota_peak_nonpaged: usize,
    quota_nonpaged: usize,
    pagefile: usize,
    peak_pagefile: usize,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn K32GetProcessMemoryInfo(
        process: *mut std::ffi::c_void,
        counters: *mut ProcessMemory,
        size: u32,
    ) -> i32;
}

fn memory(child: &Child) -> Option<ProcessMemory> {
    let mut counters = ProcessMemory {
        size: std::mem::size_of::<ProcessMemory>() as u32,
        ..Default::default()
    };
    let size = counters.size;
    // The live Child owns this process handle; the writable structure has the required ABI size.
    let ok = unsafe { K32GetProcessMemoryInfo(child.as_raw_handle(), &mut counters, size) };
    (ok != 0).then_some(counters)
}

fn bounded(mut command: Command) -> Output {
    // File capture keeps a large report/text extraction from blocking on full stdout/stderr pipes.
    let stdout = tempfile::NamedTempFile::new().unwrap();
    let stderr = tempfile::NamedTempFile::new().unwrap();
    let mut child = command
        .stdin(Stdio::null())
        .stdout(stdout.as_file().try_clone().unwrap())
        .stderr(stderr.as_file().try_clone().unwrap())
        .spawn()
        .expect("real-corpus subprocess must start");
    let start = Instant::now();
    let status = loop {
        let measured = memory(&child);
        if let Some(counters) = &measured {
            PEAK_MEMORY.fetch_max(counters.peak_working_set, Ordering::Relaxed);
            if counters.working_set > MAX_MEMORY || counters.peak_working_set > MAX_MEMORY {
                let _ = child.kill();
                let _ = child.wait();
                panic!("real-corpus subprocess exceeded the 768 MiB memory guard");
            }
        }
        if start.elapsed() > Duration::from_secs(180)
            || stdout.as_file().metadata().unwrap().len() > MAX_OUTPUT
            || stderr.as_file().metadata().unwrap().len() > MAX_OUTPUT
        {
            let _ = child.kill();
            let _ = child.wait();
            panic!("real-corpus subprocess exceeded its 180-second/2-MiB-output guard");
        }
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if measured.is_none() {
            let _ = child.kill();
            let _ = child.wait();
            panic!("memory counters unavailable for a running real-corpus subprocess");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    Output {
        status,
        stdout: std::fs::read(stdout.path()).unwrap(),
        stderr: std::fs::read(stderr.path()).unwrap(),
    }
}

fn cli(subcommand: &str, file: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
    command.arg(subcommand).arg(file);
    command
}

fn success(command: Command) -> serde_json::Value {
    let output = bounded(command);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn sha256(path: &Path) -> String {
    let mut command = Command::new("powershell.exe");
    command.args(["-NoProfile", "-NonInteractive", "-Command",
        "$s=[IO.File]::OpenRead($env:SUNIPDF_CORPUS_HASH_FILE);$h=[Security.Cryptography.SHA256]::Create();try{[BitConverter]::ToString($h.ComputeHash($s)).Replace('-','')}finally{$s.Dispose();$h.Dispose()}"])
        .env("SUNIPDF_CORPUS_HASH_FILE", path);
    let output = bounded(command);
    assert!(
        output.status.success(),
        "SHA256 process failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn qpdf_output(qpdf: &Path, file: &Path) -> Output {
    let mut command = Command::new(qpdf);
    command.arg("--check").arg(file);
    bounded(command)
}

fn qpdf_clean(qpdf: &Path, file: &Path) {
    let output = qpdf_output(qpdf, file);
    assert_eq!(
        output.status.code(),
        Some(0),
        "rewritten {}: {}",
        file.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "qpdf output must have no warnings"
    );
}

fn qpdf_source(qpdf: &Path, file: &Path, entry: &serde_json::Value) {
    let output = qpdf_output(qpdf, file);
    assert_eq!(
        output.status.code(),
        Some(entry["qpdf_source_exit"].as_i64().unwrap() as i32)
    );
    let diagnostics = String::from_utf8(output.stderr)
        .unwrap()
        .replace(&file.display().to_string(), "<input>")
        .replace("\r\n", "\n");
    let mut saved = tempfile::NamedTempFile::new().unwrap();
    saved.write_all(diagnostics.as_bytes()).unwrap();
    saved.flush().unwrap();
    let saved = saved.into_temp_path();
    assert_eq!(
        sha256(&saved),
        entry["qpdf_source_diagnostics_sha256"].as_str().unwrap(),
        "original qpdf diagnostics changed: {}",
        entry["id"]
    );
}

fn text(file: &Path, page: u64) -> Vec<u8> {
    let mut command = cli("text", file);
    command.arg("--page").arg(page.to_string());
    let output = bounded(command);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn render(file: &Path, page: u64, out: &Path) -> image::RgbaImage {
    let mut command = cli("render", file);
    command
        .args(["--page", &page.to_string(), "--scale", "0.4", "--out"])
        .arg(out);
    success(command);
    let image = image::open(out).unwrap().into_rgba8();
    assert!(
        image.width() > 100
            && image.height() > 100
            && image.width() < 2000
            && image.height() < 2000
    );
    assert!(
        image
            .pixels()
            .filter(|p| p[0] < 180 || p[1] < 180 || p[2] < 180)
            .count()
            > 30,
        "sample page rendered blank"
    );
    image
}

#[test]
#[ignore = "requires scripts/fetch-regression-corpus.ps1 and pinned qpdf; run explicitly in Windows CI"]
fn licensed_real_documents_preserve_sources_and_pass_bounded_independent_regressions() {
    let started = Instant::now();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../../scripts/regression-corpus.json")).unwrap();
    let corpus = PathBuf::from(
        std::env::var_os("SUNIPDF_REAL_CORPUS").expect("run scripts/fetch-regression-corpus.ps1"),
    );
    let qpdf = PathBuf::from(std::env::var_os("SUNIPDF_QPDF").expect("run scripts/fetch-qpdf.ps1"));
    let mut version = Command::new(&qpdf);
    version.arg("--version");
    assert!(String::from_utf8(bounded(version).stdout)
        .unwrap()
        .starts_with("qpdf version 12.4.2"));
    let temp = tempfile::tempdir().unwrap();
    let mut report = Vec::new();
    for entry in manifest["documents"].as_array().unwrap() {
        let name = entry["filename"].as_str().unwrap();
        assert!(Path::new(name).file_name().unwrap() == name && name.ends_with(".pdf"));
        let source = std::path::absolute(corpus.join(name)).unwrap();
        assert_eq!(
            std::fs::metadata(&source).unwrap().len(),
            entry["bytes"].as_u64().unwrap()
        );
        assert!(entry["bytes"].as_u64().unwrap() <= 16 * 1024 * 1024);
        let original = std::fs::read(&source).unwrap();
        assert_eq!(original.len() as u64, entry["bytes"].as_u64().unwrap());
        assert!(original.len() <= 16 * 1024 * 1024);
        assert_eq!(sha256(&source), entry["sha256"].as_str().unwrap());
        qpdf_source(&qpdf, &source, entry);
        let info = success(cli("info", &source));
        assert_eq!(info["page_count"], entry["page_count"]);
        let page = entry["sample_page"].as_u64().unwrap();
        let sample = text(&source, page);
        let sample_text = String::from_utf8_lossy(&sample);
        assert!(
            sample_text.contains(entry["text_marker"].as_str().unwrap()),
            "unexpected text from {name}: {sample_text}"
        );
        let image = render(
            &source,
            page,
            &temp.path().join(format!("{name}.original.png")),
        );

        let extracted = temp.path().join(format!("{name}.extract.pdf"));
        let mut command = cli("extract", &source);
        command
            .args([
                "--pages",
                &format!("{page},{}", entry["page_count"]),
                "--out",
            ])
            .arg(&extracted);
        assert_eq!(success(command)["page_count"], 2);
        qpdf_clean(&qpdf, &extracted);
        assert_eq!(text(&extracted, 1), sample);

        let optimized = temp.path().join(format!("{name}.optimized.pdf"));
        let mut command = cli("optimize", &source);
        command.arg("--out").arg(&optimized);
        let optimize_report = success(command);
        if optimize_report["before"] == optimize_report["after"] {
            assert_eq!(
                std::fs::read(&optimized).unwrap(),
                original,
                "no-gain optimization must retain the original exactly"
            );
            qpdf_source(&qpdf, &optimized, entry);
        } else {
            qpdf_clean(&qpdf, &optimized);
        }
        assert_eq!(success(cli("info", &optimized))["pages"], info["pages"]);
        assert_eq!(text(&optimized, page), sample);
        assert_eq!(
            render(
                &optimized,
                page,
                &temp.path().join(format!("{name}.optimized.png"))
            ),
            image,
            "lossless optimization changed rendered pixels"
        );

        let compressed = temp.path().join(format!("{name}.compressed.pdf"));
        let mut command = cli("compress-images", &source);
        command
            .args(["--max-image-side", "512", "--jpeg-quality", "65", "--out"])
            .arg(&compressed);
        let compress_report = success(command);
        assert!(std::fs::metadata(&compressed).unwrap().len() <= original.len() as u64);
        if compress_report["images_changed"].as_u64().unwrap() > 0 {
            qpdf_clean(&qpdf, &compressed);
        } else {
            assert_eq!(
                std::fs::read(&compressed).unwrap(),
                original,
                "no-gain compression must retain the original exactly"
            );
            qpdf_source(&qpdf, &compressed, entry);
        }
        if entry["compression_expected"] == true {
            assert!(
                compress_report["images_changed"].as_u64().unwrap() >= 1,
                "real grayscale photographs were not compressed"
            );
            assert!(std::fs::metadata(&compressed).unwrap().len() < original.len() as u64);
        }
        assert_eq!(success(cli("info", &compressed))["pages"], info["pages"]);
        assert_eq!(text(&compressed, page), sample);
        render(
            &compressed,
            page,
            &temp.path().join(format!("{name}.compressed.png")),
        );

        let corrupt = temp.path().join(format!("{name}.corrupt.pdf"));
        std::fs::write(&corrupt, &original[..12]).unwrap();
        let rejected = temp.path().join(format!("{name}.rejected.pdf"));
        let mut command = cli("compress-images", &corrupt);
        command.arg("--out").arg(&rejected);
        let failure = bounded(command);
        assert!(!failure.status.success());
        let error: serde_json::Value = serde_json::from_slice(&failure.stderr)
            .expect("typed CLI error instead of process crash");
        assert!(error["error"].is_string());
        assert!(!rejected.exists());
        assert_eq!(std::fs::read(&corrupt).unwrap(), original[..12]);
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert_eq!(sha256(&source), entry["sha256"].as_str().unwrap());
        report.push(serde_json::json!({"id":entry["id"],"source_sha256":entry["sha256"],"page_count":entry["page_count"],
            "original_qpdf_exit":entry["qpdf_source_exit"],"original_diagnostics_pinned":true,"source_unchanged":true,
            "optimize":optimize_report,"compress":compress_report,"corrupt_input_rejected":true}));
        println!(
            "Real corpus passed: {} ({} pages)",
            entry["id"], entry["page_count"]
        );
    }
    let report = serde_json::json!({"documents":report,"elapsed_ms":started.elapsed().as_millis(),
        "peak_child_working_set_bytes":PEAK_MEMORY.load(Ordering::Relaxed),"memory_guard_bytes":MAX_MEMORY,
        "process_timeout_seconds":180,"output_limit_bytes":MAX_OUTPUT,"qpdf_version":"12.4.2",
        "corpus_distributed":false,"license_policy_url":manifest["license_policy_url"]});
    std::fs::write(
        root.join("target/regression-corpus-report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
