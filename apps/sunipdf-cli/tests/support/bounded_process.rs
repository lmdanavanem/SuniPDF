#![allow(dead_code)]
use std::os::windows::io::AsRawHandle;
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex,
};
use std::time::{Duration, Instant};

pub const MAX_MEMORY: usize = 768 * 1024 * 1024;
pub const MAX_OUTPUT: u64 = 2 * 1024 * 1024;
pub static PEAK_MEMORY: AtomicUsize = AtomicUsize::new(0);
pub static COMMAND_METRICS: Mutex<Vec<serde_json::Value>> = Mutex::new(Vec::new());

pub fn hardware() -> serde_json::Value {
    serde_json::json!({"os":std::env::consts::OS,"architecture":std::env::consts::ARCH,
        "processor_identifier":std::env::var("PROCESSOR_IDENTIFIER").ok(),
        "logical_processors":std::thread::available_parallelism().map(usize::from).ok(),
        "runner_image":std::env::var("ImageOS").ok(),"image_version":std::env::var("ImageVersion").ok(),
        "source_revision":std::env::var("GITHUB_SHA").ok(),"run_id":std::env::var("GITHUB_RUN_ID").ok(),
        "ci":std::env::var_os("CI").is_some()})
}

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

pub fn bounded(command: Command) -> Output {
    bounded_for(command, Duration::from_secs(180))
}

pub fn bounded_for(mut command: Command, timeout: Duration) -> Output {
    // File capture keeps a large report/text extraction from blocking on full stdout/stderr pipes.
    let stdout = tempfile::NamedTempFile::new().unwrap();
    let stderr = tempfile::NamedTempFile::new().unwrap();
    let operation = command
        .get_args()
        .next()
        .map(|arg| arg.to_string_lossy().into_owned());
    let mut peak = 0;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(stdout.as_file().try_clone().unwrap())
        .stderr(stderr.as_file().try_clone().unwrap())
        .spawn()
        .expect("QA subprocess must start");
    let start = Instant::now();
    let status = loop {
        let measured = memory(&child);
        if let Some(counters) = &measured {
            PEAK_MEMORY.fetch_max(counters.peak_working_set, Ordering::Relaxed);
            peak = peak.max(counters.peak_working_set);
            if counters.working_set > MAX_MEMORY || counters.peak_working_set > MAX_MEMORY {
                let _ = child.kill();
                let _ = child.wait();
                panic!("QA subprocess exceeded the 768 MiB memory guard");
            }
        }
        if start.elapsed() > timeout
            || stdout.as_file().metadata().unwrap().len() > MAX_OUTPUT
            || stderr.as_file().metadata().unwrap().len() > MAX_OUTPUT
        {
            let _ = child.kill();
            let _ = child.wait();
            panic!("QA subprocess exceeded its time/output guard");
        }
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if measured.is_none() {
            let _ = child.kill();
            let _ = child.wait();
            panic!("memory counters unavailable for a running QA subprocess");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    COMMAND_METRICS.lock().unwrap().push(serde_json::json!({"operation":operation,"elapsed_ms":start.elapsed().as_millis(),"exit_code":status.code(),"peak_working_set_bytes":(peak != 0).then_some(peak)}));
    Output {
        status,
        stdout: std::fs::read(stdout.path()).unwrap(),
        stderr: std::fs::read(stderr.path()).unwrap(),
    }
}

pub fn sha256(path: &Path) -> String {
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
