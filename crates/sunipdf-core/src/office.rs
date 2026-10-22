//! Installed local LibreOffice only, isolated profiles, guarded inputs and new-copy PDF outputs.

use crate::{Engine, EngineError};
use std::{
    fs,
    path::{Component, Path, PathBuf},
    sync::atomic::AtomicBool,
};
#[cfg(windows)]
use std::{
    io::Read,
    process::{Child, Command, Stdio},
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

#[cfg(windows)]
const MAX_INPUT: u64 = 64 * 1024 * 1024;
#[cfg(windows)]
const MAX_PDF: u64 = 256 * 1024 * 1024;
#[cfg(windows)]
const INPUT_GUARD: &str = include_str!("office_input_guard.ps1");

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Profile {
    #[default]
    Standard,
    Print,
}
impl Profile {
    pub fn id(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Print => "print",
        }
    }
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "standard" => Some(Self::Standard),
            "print" => Some(Self::Print),
            _ => None,
        }
    }
    #[cfg(any(windows, test))]
    fn filter_options(self) -> &'static str {
        match self {
            Self::Standard => {
                r#"{"SelectPdfVersion":{"type":"long","value":"0"},"UseLosslessCompression":{"type":"boolean","value":"false"},"Quality":{"type":"long","value":"90"},"ReduceImageResolution":{"type":"boolean","value":"true"},"MaxImageResolution":{"type":"long","value":"150"},"ExportFormFields":{"type":"boolean","value":"false"}}"#
            }
            Self::Print => {
                r#"{"SelectPdfVersion":{"type":"long","value":"0"},"UseLosslessCompression":{"type":"boolean","value":"true"},"ReduceImageResolution":{"type":"boolean","value":"false"},"ExportFormFields":{"type":"boolean","value":"false"}}"#
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Converter {
    pub path: PathBuf,
}
impl Converter {
    pub fn detect() -> Option<Self> {
        #[cfg(windows)]
        for variable in ["ProgramFiles", "ProgramFiles(x86)"] {
            if let Some(root) = std::env::var_os(variable) {
                let path = PathBuf::from(root).join("LibreOffice/program/soffice.com");
                if path.is_file() && validate_local_path(&path).is_ok() {
                    return Some(Self { path });
                }
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct ConversionReport {
    pub profile: Profile,
    pub pages: usize,
    pub input_bytes: usize,
    pub output_bytes: usize,
    pub converter_version: String,
}
impl ConversionReport {
    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({"profile":self.profile.id(),"pages":self.pages,"input_bytes":self.input_bytes,"output_bytes":self.output_bytes,"converter_version":self.converter_version,"original_unchanged":true,"processing":"local isolated LibreOffice; macro and external-content inputs refused"})
    }
}
fn error(message: &str) -> EngineError {
    EngineError::Other(message.into())
}
#[cfg(windows)]
fn check_cancel(cancel: &AtomicBool) -> Result<(), EngineError> {
    if cancel.load(Ordering::Relaxed) {
        Err(EngineError::Cancelled)
    } else {
        Ok(())
    }
}

fn validate_local_path(path: &Path) -> Result<(), EngineError> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(error(
            "Office paths must be absolute local paths without traversal",
        ));
    }
    #[cfg(windows)]
    {
        use std::{os::windows::fs::MetadataExt, path::Prefix};
        let drive = match path.components().next() {
            Some(Component::Prefix(p)) => match p.kind() {
                Prefix::Disk(drive) => drive,
                _ => return Err(error("Office conversion refuses network or device paths")),
            },
            _ => return Err(error("Office conversion requires a local drive")),
        };
        let root = [u16::from(drive), b':' as u16, b'\\' as u16, 0];
        // SAFETY: root is a terminated drive root; the API only reads it.
        if !matches!(
            unsafe {
                windows::Win32::Storage::FileSystem::GetDriveTypeW(windows::core::PCWSTR(
                    root.as_ptr(),
                ))
            },
            2 | 3 | 6
        ) {
            return Err(error(
                "Office conversion requires a local removable, fixed or RAM drive",
            ));
        }
        if path
            .components()
            .any(|c| matches!(c, Component::Normal(n) if n.to_string_lossy().contains(':')))
        {
            return Err(error("Office conversion refuses alternate data streams"));
        }
        for ancestor in path.ancestors() {
            match fs::symlink_metadata(ancestor) {
                Ok(meta) if meta.file_attributes() & 0x400 != 0 => {
                    return Err(error("Office conversion refuses reparse-point paths"))
                }
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound && ancestor == path => {}
                Err(e) => return Err(e.into()),
            }
        }
    }
    #[cfg(not(windows))]
    for ancestor in path.ancestors() {
        if fs::symlink_metadata(ancestor).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(error("Office conversion refuses symbolic-link paths"));
        }
    }
    Ok(())
}
pub(crate) fn validate_local_output(path: &Path) -> Result<(), EngineError> {
    validate_local_path(path)?;
    if !path.parent().is_some_and(Path::is_dir) || path.file_name().is_none() {
        return Err(error("Office output requires an existing local directory"));
    }
    Ok(())
}
#[cfg(windows)]
fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, EngineError> {
    validate_local_path(path)?;
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000);
    }
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(error("Office input is not a regular local file"));
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(error("Office input changed to a reparse point"));
        }
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(error("Office file exceeds its size limit"));
    }
    Ok(bytes)
}
#[cfg(windows)]
fn hidden(command: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}
#[cfg(windows)]
struct Running(Child);
#[cfg(windows)]
impl Drop for Running {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            #[cfg(windows)]
            if let Ok(tool) = system_tool("taskkill.exe") {
                let _ = hidden(&mut Command::new(tool))
                    .args(["/PID", &self.0.id().to_string(), "/T", "/F"])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}
#[cfg(windows)]
fn system_tool(name: &str) -> Result<PathBuf, EngineError> {
    #[cfg(windows)]
    {
        let mut buffer = [0u16; 32768];
        // SAFETY: the OS receives the complete writable buffer length.
        let len = unsafe {
            windows::Win32::System::SystemInformation::GetSystemDirectoryW(Some(&mut buffer))
        } as usize;
        if len > 0 && len < buffer.len() {
            return Ok(PathBuf::from(String::from_utf16_lossy(&buffer[..len])).join(name));
        }
    }
    let _ = name;
    Err(error("Trusted Windows system directory is unavailable"))
}
#[cfg(windows)]
fn run(
    mut command: Command,
    cancel: &AtomicBool,
    timeout: Duration,
    log: &Path,
    output_pdf: Option<&Path>,
) -> Result<String, EngineError> {
    check_cancel(cancel)?;
    let output = fs::File::create(log)?;
    command
        .stdin(Stdio::null())
        .stdout(output.try_clone()?)
        .stderr(output);
    let mut running = Running(hidden(&mut command).spawn()?);
    let start = Instant::now();
    loop {
        check_cancel(cancel)?;
        if fs::metadata(log)?.len() > 64 * 1024 {
            return Err(error("Local converter diagnostic limit exceeded"));
        }
        if output_pdf.is_some_and(|path| fs::metadata(path).is_ok_and(|m| m.len() > MAX_PDF)) {
            return Err(error("Local converter PDF exceeds 256 MiB"));
        }
        if let Some(status) = running.0.try_wait()? {
            let text = String::from_utf8_lossy(&read_bounded(log, 64 * 1024)?).into_owned();
            if !status.success() {
                return Err(error(&format!(
                    "Local converter refused the document: {}",
                    text.chars().take(600).collect::<String>()
                )));
            }
            return Ok(text);
        }
        if start.elapsed() >= timeout {
            return Err(error("Local converter timed out"));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
#[cfg(any(windows, test))]
fn supported_version(text: &str) -> bool {
    text.trim()
        .strip_prefix("LibreOffice ")
        .and_then(|v| v.split_whitespace().next())
        .is_some_and(|v| v.starts_with("26.2.") || v.starts_with("26.8."))
}

#[cfg(windows)]
const HEADLESS_ARGS: [&str; 6] = [
    "--headless",
    "--nologo",
    "--nodefault",
    "--nofirststartwizard",
    "--norestore",
    "--quickstart=no",
];
#[cfg(any(windows, test))]
fn file_url(path: &Path) -> Result<String, EngineError> {
    let path = path
        .to_str()
        .ok_or_else(|| error("Local converter profile path is not Unicode"))?
        .replace('\\', "/");
    let mut url = String::from("file:///");
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b':' | b'-' | b'_' | b'.' | b'~') {
            url.push(char::from(byte));
        } else {
            url.push_str(&format!("%{byte:02X}"));
        }
    }
    Ok(url)
}
#[cfg(any(windows, test))]
fn profile_config() -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><oor:items xmlns:oor="http://openoffice.org/2001/registry">"#,
    );
    xml.push_str(r#"<item oor:path="/org.openoffice.Office.Common/Security/Scripting">"#);
    for name in [
        "DisableMacrosExecution",
        "DisableActiveContent",
        "DisablePythonRuntime",
        "DisableOLEAutomation",
        "BlockUntrustedRefererLinks",
    ] {
        xml.push_str(&format!(
            r#"<prop oor:name="{name}" oor:op="fuse"><value>true</value></prop>"#
        ));
    }
    xml.push_str(r#"<prop oor:name="MacroSecurityLevel" oor:op="fuse"><value>3</value></prop></item><item oor:path="/org.openoffice.Inet/Settings"><prop oor:name="ooInetProxyType" oor:op="fuse"><value>2</value></prop>"#);
    for protocol in ["HTTP", "HTTPS"] {
        xml.push_str(&format!(r#"<prop oor:name="ooInet{protocol}ProxyName" oor:op="fuse"><value>127.0.0.1</value></prop><prop oor:name="ooInet{protocol}ProxyPort" oor:op="fuse"><value>9</value></prop>"#));
    }
    xml.push_str(
        r#"<prop oor:name="ooInetNoProxy" oor:op="fuse"><value></value></prop></item></oor:items>"#,
    );
    xml
}

#[cfg(windows)]
pub(crate) fn validate_package(
    input: &Path,
    kind: &str,
    cancel: &AtomicBool,
    stage: &Path,
) -> Result<(), EngineError> {
    let guard = stage.join("guard.ps1");
    fs::write(&guard, INPUT_GUARD)?;
    let mut command = Command::new(system_tool("WindowsPowerShell/v1.0/powershell.exe")?);
    command
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&guard)
        .arg("-InputPath")
        .arg(input)
        .arg("-Kind")
        .arg(kind);
    let safe = run(
        command,
        cancel,
        Duration::from_secs(30),
        &stage.join("guard.log"),
        None,
    )?;
    if safe != "SAFE" {
        return Err(error("Office package validation did not complete"));
    }
    Ok(())
}

pub fn convert(
    engine: &Engine,
    converter: &Converter,
    input: &Path,
    output: &Path,
    profile: Profile,
    cancel: &AtomicBool,
) -> Result<ConversionReport, EngineError> {
    #[cfg(not(windows))]
    {
        let _ = (engine, converter, input, output, profile, cancel);
        Err(error("Local Office conversion currently requires Windows"))
    }
    #[cfg(windows)]
    {
        check_cancel(cancel)?;
        validate_local_output(output)?;
        validate_local_path(&converter.path)?;
        if output.exists() {
            return Err(EngineError::AlreadyExists(output.into()));
        }
        if !output
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
        {
            return Err(error("Office conversion output must have a .pdf extension"));
        }
        let kind = input
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let filter = match kind.as_str() {
            "docx" | "odt" => "writer_pdf_Export",
            "xlsx" | "ods" => "calc_pdf_Export",
            "pptx" | "odp" => "impress_pdf_Export",
            _ => {
                return Err(error(
                    "Office conversion accepts macro-free DOCX/XLSX/PPTX/ODT/ODS/ODP only",
                ))
            }
        };
        let source = read_bounded(input, MAX_INPUT)?;
        let stage = tempfile::tempdir()?;
        validate_local_path(stage.path())?;
        let staged_input = stage.path().join(format!("source.{kind}"));
        fs::write(&staged_input, &source)?;
        validate_package(&staged_input, &kind, cancel, stage.path())?;
        let profile_dir = stage.path().join("profile");
        fs::create_dir_all(profile_dir.join("user"))?;
        fs::write(
            profile_dir.join("user/registrymodifications.xcu"),
            profile_config(),
        )?;
        let profile_arg = format!("-env:UserInstallation={}", file_url(&profile_dir)?);
        let mut version_command = Command::new(&converter.path);
        version_command
            .arg(&profile_arg)
            .args(HEADLESS_ARGS)
            .arg("--version");
        let version = run(
            version_command,
            cancel,
            // Cold startup on a loaded Windows machine can exceed ten seconds.
            Duration::from_secs(30),
            &stage.path().join("version.log"),
            None,
        )?;
        if !supported_version(&version) {
            return Err(error(
                "A supported installed LibreOffice 26.2 or 26.8 is required",
            ));
        }
        let out = stage.path().join("out");
        fs::create_dir(&out)?;
        let mut command = Command::new(&converter.path);
        command
            .arg(&profile_arg)
            .args(HEADLESS_ARGS)
            .arg("--convert-to")
            .arg(format!("pdf:{filter}:{}", profile.filter_options()))
            .arg("--outdir")
            .arg(&out)
            .arg(&staged_input);
        let pdf_path = out.join("source.pdf");
        run(
            command,
            cancel,
            Duration::from_secs(120),
            &stage.path().join("conversion.log"),
            Some(&pdf_path),
        )?;
        let bytes = read_bounded(&pdf_path, MAX_PDF)?;
        let doc = engine.open_bytes(bytes.clone(), None, &pdf_path)?;
        if doc.page_count() == 0 || doc.page_count() > 1000 || doc.properties().encrypted {
            return Err(error("Local converter produced an unsupported PDF"));
        }
        let pages = doc.page_count();
        drop(doc);
        if read_bounded(input, MAX_INPUT)? != source {
            return Err(error("Office source changed during conversion"));
        }
        check_cancel(cancel)?;
        validate_local_output(output)?;
        crate::write_new_copy(output, &bytes)?;
        Ok(ConversionReport {
            profile,
            pages,
            input_bytes: source.len(),
            output_bytes: bytes.len(),
            converter_version: version.trim().into(),
        })
    }
}

#[cfg(test)]
#[path = "office_tests.rs"]
mod tests;
