//! Verified platform updates. Windows installs after a guarded Quit; Ubuntu keeps documents
//! open while its authenticated package-manager transaction completes.

use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::process::Command;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};
use tempfile::TempDir;

const LATEST_URL: &str = "https://api.github.com/repos/lmdanavanem/SuniPDF/releases/latest";
const RELEASES_URL: &str = "https://github.com/lmdanavanem/SuniPDF/releases";
const MAX_RELEASE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_PACKAGE_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateKind {
    #[cfg(any(windows, test))]
    Msi,
    #[cfg(any(windows, test))]
    Portable,
    Deb,
}

#[derive(Clone, Debug)]
pub struct Release {
    pub version: String,
    pub asset_name: String,
    pub notes_url: String,
    pub kind: UpdateKind,
    asset_url: String,
    pub(crate) sha256: String,
    pub(crate) size: u64,
    #[cfg_attr(not(windows), allow(dead_code))]
    pub(crate) arch: String,
}

#[derive(Debug)]
pub struct PreparedUpdate {
    pub release: Release,
    #[cfg_attr(not(windows), allow(dead_code))]
    pub(crate) workspace: Option<TempDir>,
    #[cfg_attr(not(windows), allow(dead_code))]
    pub(crate) package: PathBuf,
    #[cfg_attr(not(windows), allow(dead_code))]
    target: PathBuf,
}

#[derive(Debug, Default)]
pub enum Phase {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available(Release),
    Downloading,
    Ready(PreparedUpdate),
    Installing(Instant),
    Installed(String),
    InstallFailed(PreparedUpdate, String),
    Failed(String),
}

#[derive(Default)]
pub struct Updater {
    pub phase: Phase,
    rx: Option<Receiver<Result<Phase, String>>>,
    #[cfg(test)]
    check_requests: Option<mpsc::Sender<()>>,
    #[cfg(test)]
    restart_requests: Option<mpsc::Sender<()>>,
}

impl Updater {
    #[cfg(test)]
    pub(crate) fn test_channels() -> (Self, Receiver<()>) {
        let (tx, rx) = mpsc::channel();
        (
            Self {
                check_requests: Some(tx),
                ..Default::default()
            },
            rx,
        )
    }

    pub fn start_check(&mut self, ctx: &egui::Context) {
        if matches!(
            self.phase,
            Phase::Checking
                | Phase::Downloading
                | Phase::Ready(_)
                | Phase::Installing(_)
                | Phase::Installed(_)
                | Phase::InstallFailed(_, _)
        ) {
            return;
        }
        #[cfg(test)]
        if let Some(tx) = &self.check_requests {
            let _ = tx.send(());
            self.phase = Phase::Checking;
            return;
        }
        self.start(ctx, Phase::Checking, check_latest);
    }

    pub fn download(&mut self, ctx: &egui::Context) {
        self.download_with(ctx, download_release);
    }

    fn download_with(
        &mut self,
        ctx: &egui::Context,
        work: impl FnOnce(Release) -> Result<PreparedUpdate, String> + Send + 'static,
    ) {
        let Phase::Available(release) = &self.phase else {
            return;
        };
        let release = release.clone();
        self.start(ctx, Phase::Downloading, move || {
            work(release).map(Phase::Ready)
        });
    }

    pub fn is_installing(&self) -> bool {
        matches!(self.phase, Phase::Installing(_))
    }

    pub fn install_prepared(&mut self, ctx: &egui::Context) {
        self.install_with(ctx, crate::updater_linux::install);
    }

    fn install_with(
        &mut self,
        ctx: &egui::Context,
        work: impl FnOnce(&PreparedUpdate) -> Result<(), String> + Send + 'static,
    ) {
        if !matches!(self.phase, Phase::Ready(ref prepared) | Phase::InstallFailed(ref prepared, _)
            if prepared.release.kind == UpdateKind::Deb)
        {
            return;
        }
        let prepared = match std::mem::take(&mut self.phase) {
            Phase::Ready(prepared) | Phase::InstallFailed(prepared, _) => prepared,
            _ => unreachable!("the phase was checked before ownership transfer"),
        };
        self.start(ctx, Phase::Installing(Instant::now()), move || {
            Ok(match work(&prepared) {
                Ok(()) => Phase::Installed(prepared.release.version.clone()),
                Err(error) => Phase::InstallFailed(prepared, error),
            })
        });
    }

    pub fn restart_installed(&self) -> Result<(), String> {
        if !matches!(self.phase, Phase::Installed(_)) {
            return Err("Install the update successfully before restarting.".into());
        }
        #[cfg(test)]
        {
            self.restart_requests
                .as_ref()
                .ok_or("No test restart receiver")?
                .send(())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(test))]
        {
            crate::updater_linux::restart()
        }
    }

    #[cfg(test)]
    pub(crate) fn test_install_channels(
    ) -> (Self, mpsc::Sender<Result<Phase, String>>, Receiver<()>) {
        let (tx, rx) = mpsc::channel();
        let (restart_tx, restart_rx) = mpsc::channel();
        (
            Self {
                phase: Phase::Installing(Instant::now()),
                rx: Some(rx),
                restart_requests: Some(restart_tx),
                ..Default::default()
            },
            tx,
            restart_rx,
        )
    }

    fn start(
        &mut self,
        ctx: &egui::Context,
        phase: Phase,
        work: impl FnOnce() -> Result<Phase, String> + Send + 'static,
    ) {
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        match std::thread::Builder::new()
            .name("sunipdf-update".into())
            .spawn(move || {
                let _ = tx.send(work());
                ctx.request_repaint();
            }) {
            Ok(_) => {
                self.phase = phase;
                self.rx = Some(rx);
            }
            Err(e) => self.phase = Phase::Failed(format!("Could not start update check: {e}")),
        }
    }

    pub fn poll(&mut self) -> bool {
        let Some(rx) = &self.rx else { return false };
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => Err("The update task stopped unexpectedly.".into()),
        };
        self.rx = None;
        self.phase = result.unwrap_or_else(Phase::Failed);
        true
    }

    /// Called only after all document-save and close confirmations have succeeded.
    #[cfg(windows)]
    pub fn launch_prepared(&mut self) -> Result<(), String> {
        let Phase::Ready(prepared) = &mut self.phase else {
            return Err("Download and verify the update before installing it.".into());
        };
        let workspace = prepared
            .workspace
            .as_ref()
            .ok_or("The update installer has already been started.")?;
        let helper = workspace.path().join("update-helper.ps1");
        fs::write(&helper, include_str!("update-helper.ps1"))
            .map_err(|e| format!("Could not prepare the update installer: {e}"))?;
        let mut command = powershell()?;
        command
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-File")
            .arg(&helper)
            .arg("-Mode")
            .arg(match prepared.release.kind {
                UpdateKind::Msi => "Msi",
                UpdateKind::Portable => "Portable",
                UpdateKind::Deb => {
                    return Err("Install Ubuntu updates with your package manager.".into())
                }
            })
            .arg("-ParentPid")
            .arg(std::process::id().to_string())
            .arg("-PackagePath")
            .arg(&prepared.package)
            .arg("-ExpectedSha256")
            .arg(&prepared.release.sha256)
            .arg("-TargetDirectory")
            .arg(&prepared.target)
            .arg("-ExecutableName")
            .arg("sunipdf.exe")
            .arg("-Arch")
            .arg(&prepared.release.arch)
            .arg("-Restart");
        command
            .spawn()
            .map_err(|e| format!("Could not start the update installer: {e}"))?;
        // The helper and rollback backup must outlive this process.
        if let Some(workspace) = prepared.workspace.take() {
            let _ = workspace.keep();
        }
        Ok(())
    }

    #[cfg(not(windows))]
    pub fn launch_prepared(&mut self) -> Result<(), String> {
        Err("Install Ubuntu updates with your package manager.".into())
    }
}

fn check_latest() -> Result<Phase, String> {
    #[cfg(windows)]
    let exe = fs::canonicalize(std::env::current_exe().map_err(|e| e.to_string())?)
        .map_err(|e| format!("Could not locate this application: {e}"))?;
    let arch = architecture(std::env::consts::ARCH)?;
    #[cfg(windows)]
    let kind = if arch == "x64" && is_installed(&exe) {
        UpdateKind::Msi
    } else {
        UpdateKind::Portable
    };
    #[cfg(not(windows))]
    let kind = UpdateKind::Deb;
    let mut response = https_get(
        LATEST_URL,
        "application/vnd.github+json",
        Duration::from_secs(20),
    )?;
    let body = read_bounded(response.body_mut().as_reader(), MAX_RELEASE_BYTES)?;
    Ok(
        match parse_release(&body, env!("CARGO_PKG_VERSION"), arch, kind)? {
            Some(release) => Phase::Available(release),
            None => Phase::UpToDate,
        },
    )
}

fn architecture(arch: &str) -> Result<&'static str, String> {
    architecture_for(std::env::consts::OS, arch)
}

fn architecture_for(platform: &str, arch: &str) -> Result<&'static str, String> {
    match (platform, arch) {
        ("windows", "x86_64") => Ok("x64"),
        ("windows", "aarch64") => Ok("arm64"),
        ("linux", "x86_64") => Ok("amd64"),
        _ => Err("Updates are available for Windows x64/ARM64 and Ubuntu amd64.".into()),
    }
}

#[derive(Deserialize)]
struct ReleaseResponse {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    state: String,
    size: u64,
    digest: Option<String>,
}

fn parse_release(
    body: &[u8],
    current: &str,
    arch: &str,
    kind: UpdateKind,
) -> Result<Option<Release>, String> {
    if body.len() as u64 > MAX_RELEASE_BYTES {
        return Err("The release response is too large.".into());
    }
    let supported_architecture = match kind {
        #[cfg(any(windows, test))]
        UpdateKind::Msi => arch == "x64",
        #[cfg(any(windows, test))]
        UpdateKind::Portable => matches!(arch, "x64" | "arm64"),
        UpdateKind::Deb => arch == "amd64",
    };
    if !supported_architecture {
        return Err("No installer is available for this architecture.".into());
    }
    let response: ReleaseResponse =
        serde_json::from_slice(body).map_err(|e| format!("Invalid release response: {e}"))?;
    if response.draft || response.prerelease {
        return Ok(None);
    }
    let tag = &response.tag_name;
    let version = tag
        .strip_prefix('v')
        .and_then(|v| Version::parse(v).ok())
        .filter(|v| v.pre.is_empty() && v.build.is_empty() && format!("v{v}") == *tag)
        .ok_or("The release has an invalid stable version.")?;
    let current = Version::parse(current).map_err(|e| format!("Invalid app version: {e}"))?;
    if version <= current {
        return Ok(None);
    }
    let version = version.to_string();
    let asset_name = match kind {
        #[cfg(any(windows, test))]
        UpdateKind::Msi => format!("SuniPDF-{version}-win-{arch}.msi"),
        #[cfg(any(windows, test))]
        UpdateKind::Portable => format!("SuniPDF-{version}-win-{arch}-portable.zip"),
        UpdateKind::Deb => format!("SuniPDF-{version}-linux-{arch}.deb"),
    };
    let mut assets = response.assets.into_iter().filter(|a| a.name == asset_name);
    let asset = assets
        .next()
        .ok_or_else(|| format!("This release has no {asset_name} update."))?;
    if assets.next().is_some() || asset.state != "uploaded" {
        return Err("The update package is not fully published.".into());
    }
    if asset.size == 0 || asset.size > MAX_PACKAGE_BYTES {
        return Err("The update package has an invalid size.".into());
    }
    let asset_url = format!("{RELEASES_URL}/download/{tag}/{asset_name}");
    if asset.browser_download_url != asset_url {
        return Err("The update package is not on the official release address.".into());
    }
    let sha256 = asset
        .digest
        .as_deref()
        .and_then(|d| d.strip_prefix("sha256:"))
        .filter(|d| d.len() == 64 && d.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or("The update package has no valid SHA256 fingerprint.")?
        .to_ascii_lowercase();
    Ok(Some(Release {
        notes_url: format!("{RELEASES_URL}/tag/{tag}"),
        version,
        asset_name,
        kind,
        asset_url,
        sha256,
        size: asset.size,
        arch: arch.into(),
    }))
}

fn trusted_url(url: &str) -> bool {
    let Ok(uri) = url.parse::<ureq::http::Uri>() else {
        return false;
    };
    let (Some(host), Some(authority)) = (uri.host(), uri.authority()) else {
        return false;
    };
    uri.scheme_str() == Some("https")
        && matches!(
            host,
            "api.github.com"
                | "github.com"
                | "release-assets.githubusercontent.com"
                | "objects.githubusercontent.com"
        )
        && (authority.as_str() == host || authority.as_str() == format!("{host}:443"))
        && !url.contains('#')
}

fn https_get(
    url: &str,
    accept: &str,
    timeout: Duration,
) -> Result<ureq::http::Response<ureq::Body>, String> {
    let deadline = Instant::now() + timeout;
    let mut url = url.to_owned();
    for _ in 0..=3 {
        if !trusted_url(&url) {
            return Err("The update request left the official HTTPS download servers.".into());
        }
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or("The update request timed out.")?;
        let agent = ureq::Agent::config_builder()
            .https_only(true)
            .max_redirects(0)
            .http_status_as_error(false)
            .timeout_global(Some(remaining))
            .timeout_connect(Some(Duration::from_secs(10).min(remaining)))
            .max_response_header_size(32 * 1024)
            .user_agent(concat!("SuniPDF/", env!("CARGO_PKG_VERSION")))
            .build()
            .new_agent();
        let response = agent
            .get(&url)
            .header("Accept", accept)
            .header("Accept-Encoding", "identity")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .call()
            .map_err(|e| format!("Could not contact the update server: {e}"))?;
        if response.status().is_redirection() {
            url = response
                .headers()
                .get("Location")
                .and_then(|h| h.to_str().ok())
                .ok_or("The update server returned an invalid redirect.")?
                .to_owned();
        } else if response.status().as_u16() == 200 {
            return Ok(response);
        } else {
            return Err(format!(
                "The update server returned HTTP {}.",
                response.status()
            ));
        }
    }
    Err("The update server returned too many redirects.".into())
}

fn read_bounded(reader: impl Read, limit: u64) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    reader
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Could not read the update response: {e}"))?;
    if bytes.len() as u64 > limit {
        return Err("The update response exceeds the permitted size.".into());
    }
    Ok(bytes)
}

fn download_release(release: Release) -> Result<PreparedUpdate, String> {
    if release.kind == UpdateKind::Deb {
        crate::updater_linux::require_system_installation()?;
    }
    let workspace = tempfile::Builder::new()
        .prefix("SuniPDF-update-")
        .tempdir()
        .map_err(|e| format!("Could not create the update folder: {e}"))?;
    let package = workspace.path().join(&release.asset_name);
    let exe = fs::canonicalize(std::env::current_exe().map_err(|e| e.to_string())?)
        .map_err(|e| format!("Could not locate this application: {e}"))?;
    if release.kind != UpdateKind::Deb
        && !exe
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.eq_ignore_ascii_case("sunipdf.exe"))
    {
        return Err("Automatic installation requires the original sunipdf.exe filename.".into());
    }
    let target = ordinary_path(
        exe.parent()
            .ok_or("The application folder was not found.")?,
    );
    let mut response = https_get(
        &release.asset_url,
        "application/octet-stream",
        Duration::from_secs(300),
    )?;
    if let Some(length) = response.headers().get("Content-Length") {
        if length.to_str().ok().and_then(|v| v.parse::<u64>().ok()) != Some(release.size) {
            return Err("The update download size does not match the release.".into());
        }
    }
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&package)
        .map_err(|e| format!("Could not create the update package: {e}"))?;
    copy_verified(
        response.body_mut().as_reader(),
        &mut file,
        release.size,
        &release.sha256,
    )?;
    file.sync_all()
        .map_err(|e| format!("Could not finish writing the update: {e}"))?;
    drop(file);
    Ok(PreparedUpdate {
        release,
        workspace: Some(workspace),
        package,
        target,
    })
}

fn copy_verified(
    mut reader: impl Read,
    mut writer: impl Write,
    expected_size: u64,
    expected_hash: &str,
) -> Result<(), String> {
    if expected_size == 0 || expected_size > MAX_PACKAGE_BYTES {
        return Err("The update package has an invalid size.".into());
    }
    let mut hash = Sha256::new();
    let mut size = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|e| format!("Update download interrupted: {e}"))?;
        if count == 0 {
            break;
        }
        size += count as u64;
        if size > expected_size {
            return Err("The update download exceeds the release size.".into());
        }
        hash.update(&buffer[..count]);
        writer
            .write_all(&buffer[..count])
            .map_err(|e| format!("Could not write the update package: {e}"))?;
    }
    if size != expected_size || hex(&hash.finalize()) != expected_hash {
        return Err(
            "The update package failed size or SHA256 verification. Nothing was installed.".into(),
        );
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
pub(crate) fn test_prepared_deb() -> PreparedUpdate {
    let workspace = tempfile::Builder::new()
        .prefix("SuniPDF update $ ; ")
        .tempdir()
        .unwrap();
    let version = "0.74.0".to_string();
    let asset_name = format!("SuniPDF-{version}-linux-amd64.deb");
    let package = workspace.path().join(&asset_name);
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(&package).unwrap().write_all(b"abc").unwrap();
    PreparedUpdate {
        release: Release {
            version,
            asset_name,
            notes_url: format!("{RELEASES_URL}/tag/v0.74.0"),
            kind: UpdateKind::Deb,
            asset_url: format!("{RELEASES_URL}/download/v0.74.0/SuniPDF-0.74.0-linux-amd64.deb"),
            sha256: hex(&Sha256::digest(b"abc")),
            size: 3,
            arch: "amd64".into(),
        },
        workspace: Some(workspace),
        package,
        target: PathBuf::from("/usr/lib/sunipdf"),
    }
}

#[cfg(windows)]
fn is_installed(exe: &Path) -> bool {
    let default_install = ["ProgramW6432", "ProgramFiles"]
        .into_iter()
        .filter_map(std::env::var_os)
        .map(|p| PathBuf::from(p).join("SuniPDF").join("sunipdf.exe"))
        .any(|p| same_file(&p, exe));
    if default_install {
        return true;
    }
    let Ok(mut command) = powershell() else {
        return false;
    };
    // Read the installer-owned association; no file path or server data is evaluated as script.
    let output = command.arg("-Command").arg(
        "[Console]::OutputEncoding=[Text.UTF8Encoding]::new(); $key=[Microsoft.Win32.Registry]::LocalMachine.OpenSubKey('Software\\Classes\\Applications\\sunipdf.exe\\shell\\open\\command'); if($key){try{[Console]::Write([string]$key.GetValue(''))}finally{$key.Dispose()}}"
    ).output();
    let Ok(output) = output else { return false };
    if !output.status.success() {
        return false;
    }
    let value = String::from_utf8_lossy(&output.stdout);
    let Some(path) = value
        .trim()
        .strip_prefix('"')
        .and_then(|v| v.split('"').next())
    else {
        return false;
    };
    same_file(Path::new(path), exe)
}

#[cfg(windows)]
fn same_file(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => {
            left.to_string_lossy().to_lowercase() == right.to_string_lossy().to_lowercase()
        }
        _ => false,
    }
}

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

#[cfg(windows)]
fn powershell() -> Result<Command, String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let system_root =
            std::env::var_os("SystemRoot").ok_or("The Windows folder was not found.")?;
        let mut command = Command::new(
            PathBuf::from(system_root).join("System32\\WindowsPowerShell\\v1.0\\powershell.exe"),
        );
        command.args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden"]);
        command.creation_flags(0x0800_0000);
        Ok(command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn release_body(tag: &str, arch: &str, kind: UpdateKind) -> serde_json::Value {
        let version = tag.trim_start_matches('v');
        let name = match kind {
            UpdateKind::Msi => format!("SuniPDF-{version}-win-{arch}.msi"),
            UpdateKind::Portable => format!("SuniPDF-{version}-win-{arch}-portable.zip"),
            UpdateKind::Deb => format!("SuniPDF-{version}-linux-{arch}.deb"),
        };
        json!({
            "tag_name":tag,"draft":false,"prerelease":false,
            "assets":[{"name":name,"browser_download_url":format!("{RELEASES_URL}/download/{tag}/{name}"),
                "state":"uploaded","size":3,"digest":format!("sha256:{}",hex(&Sha256::digest(b"abc")))}]
        })
    }

    fn parse(
        value: serde_json::Value,
        current: &str,
        arch: &str,
        kind: UpdateKind,
    ) -> Result<Option<Release>, String> {
        parse_release(&serde_json::to_vec(&value).unwrap(), current, arch, kind)
    }

    #[test]
    fn newer_stable_release_selects_only_exact_installation_and_architecture() {
        for (arch, kind) in [
            ("x64", UpdateKind::Msi),
            ("x64", UpdateKind::Portable),
            ("arm64", UpdateKind::Portable),
            ("amd64", UpdateKind::Deb),
        ] {
            let release = parse(release_body("v0.10.0", arch, kind), "0.9.0", arch, kind)
                .unwrap()
                .unwrap();
            assert_eq!(release.version, "0.10.0");
            assert_eq!(release.kind, kind);
            assert_eq!(release.arch, arch);
        }
        assert!(parse(
            release_body("v0.8.1", "x64", UpdateKind::Msi),
            "0.8.2",
            "x64",
            UpdateKind::Msi
        )
        .unwrap()
        .is_none());
        assert!(parse(
            release_body("v0.8.2", "x64", UpdateKind::Msi),
            "0.8.2",
            "x64",
            UpdateKind::Msi
        )
        .unwrap()
        .is_none());
        assert!(parse(
            release_body("v0.9.0", "arm64", UpdateKind::Portable),
            "0.8.2",
            "x64",
            UpdateKind::Portable
        )
        .is_err());
        assert!(parse(
            release_body("v0.9.0", "arm64", UpdateKind::Msi),
            "0.8.2",
            "arm64",
            UpdateKind::Msi
        )
        .is_err());
        assert!(architecture("x86").is_err());
    }

    #[test]
    fn ubuntu_updates_never_select_windows_packages_or_unsupported_architectures() {
        assert_eq!(architecture_for("linux", "x86_64").unwrap(), "amd64");
        assert!(architecture_for("linux", "aarch64").is_err());
        assert!(architecture_for("macos", "x86_64").is_err());
        for kind in [UpdateKind::Msi, UpdateKind::Portable] {
            assert!(parse(
                release_body("v0.52.0", "x64", kind),
                "0.51.0",
                "amd64",
                UpdateKind::Deb
            )
            .is_err());
            assert!(parse(
                release_body("v0.52.0", "amd64", kind),
                "0.51.0",
                "amd64",
                kind
            )
            .is_err());
        }
        let release = parse(
            release_body("v0.52.0", "amd64", UpdateKind::Deb),
            "0.51.0",
            "amd64",
            UpdateKind::Deb,
        )
        .unwrap()
        .unwrap();
        assert_eq!(release.asset_name, "SuniPDF-0.52.0-linux-amd64.deb");
        let ctx = egui::Context::default();
        let mut updater = Updater {
            phase: Phase::Available(release),
            ..Default::default()
        };
        let mut output = ctx.run_ui(egui::RawInput::default(), |_| {
            updater.download_with(&ctx, |release| {
                assert_eq!(release.kind, UpdateKind::Deb);
                Err("Synthetic download failure".into())
            })
        });
        output.textures_delta.clear();
        assert!(matches!(updater.phase, Phase::Downloading));
        assert!(updater.rx.is_some());
        assert!(!output
            .platform_output
            .commands
            .iter()
            .any(|command| matches!(command, egui::OutputCommand::OpenUrl(_))));
        wait_update(&mut updater);
        assert!(
            matches!(updater.phase, Phase::Failed(ref error) if error == "Synthetic download failure")
        );
    }

    fn wait_update(updater: &mut Updater) {
        let deadline = Instant::now() + Duration::from_secs(3);
        while !updater.poll() {
            assert!(Instant::now() < deadline, "update test worker timed out");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn ubuntu_install_failure_retains_verified_package_and_success_cleans_it_after_retry() {
        let ctx = egui::Context::default();
        let (mut updater, checks) = Updater::test_channels();
        let prepared = test_prepared_deb();
        let package = prepared.package.clone();
        updater.phase = Phase::Ready(prepared);
        updater.install_with(&ctx, |_| Err("Authorization cancelled".into()));
        assert!(updater.is_installing());
        updater.start_check(&ctx);
        assert!(checks.try_recv().is_err());
        wait_update(&mut updater);
        assert!(
            matches!(updater.phase, Phase::InstallFailed(_, ref error) if error == "Authorization cancelled")
        );
        assert_eq!(fs::read(&package).unwrap(), b"abc");
        updater.start_check(&ctx);
        assert!(checks.try_recv().is_err());
        updater.install_with(&ctx, |prepared| {
            assert_eq!(fs::read(&prepared.package).unwrap(), b"abc");
            Ok(())
        });
        assert!(updater.is_installing());
        wait_update(&mut updater);
        assert!(matches!(updater.phase, Phase::Installed(ref version) if version == "0.74.0"));
        assert!(!package.exists());
        updater.start_check(&ctx);
        assert!(checks.try_recv().is_err());
    }

    #[test]
    fn rejects_invalid_versions_sources_fingerprints_and_unfinished_assets() {
        for tag in [
            "0.9.0",
            "v0.9.0-beta.1",
            "v0.9.0+local",
            "v00.9.0",
            "v../0.9.0",
        ] {
            assert!(
                parse(
                    release_body(tag, "x64", UpdateKind::Msi),
                    "0.8.2",
                    "x64",
                    UpdateKind::Msi
                )
                .is_err(),
                "{tag}"
            );
        }
        for (key, value) in [
            (
                "browser_download_url",
                json!("https://evil.example/update.msi"),
            ),
            ("digest", json!(null)),
            ("digest", json!("sha256:abc")),
            ("digest", json!(format!("sha256:{}", "x".repeat(64)))),
            ("size", json!(0)),
            ("size", json!(MAX_PACKAGE_BYTES + 1)),
            ("state", json!("new")),
        ] {
            let mut body = release_body("v0.9.0", "x64", UpdateKind::Msi);
            body["assets"][0][key] = value;
            assert!(
                parse(body, "0.8.2", "x64", UpdateKind::Msi).is_err(),
                "{key}"
            );
        }
        let mut body = release_body("v0.9.0", "x64", UpdateKind::Msi);
        body["prerelease"] = json!(true);
        assert!(parse(body, "0.8.2", "x64", UpdateKind::Msi)
            .unwrap()
            .is_none());
        let mut body = release_body("v0.9.0", "x64", UpdateKind::Msi);
        let duplicate = body["assets"][0].clone();
        body["assets"].as_array_mut().unwrap().push(duplicate);
        assert!(parse(body, "0.8.2", "x64", UpdateKind::Msi).is_err());
    }

    #[test]
    fn redirects_stay_on_official_https_hosts_without_credentials_or_other_ports() {
        for url in [
            LATEST_URL,
            "https://github.com/lmdanavanem/SuniPDF/releases/download/v0.9.0/a.msi",
            "https://release-assets.githubusercontent.com/a?signature=123",
            "https://objects.githubusercontent.com/a",
        ] {
            assert!(trusted_url(url), "{url}");
        }
        for url in [
            "http://github.com/a",
            "https://github.com.evil.example/a",
            "https://github.com@evil.example/a",
            "https://evil@github.com/a",
            "https://github.com:444/a",
            "https://github.com:abc/a",
            "https://github.com:/a",
            "https://github.com:65536/a",
            "file:///C:/a",
            "https://github.com/a#b",
            "/relative",
        ] {
            assert!(!trusted_url(url), "{url}");
        }
    }

    #[test]
    fn response_and_package_reads_are_bounded_and_integrity_is_required() {
        assert_eq!(read_bounded(&b"abc"[..], 3).unwrap(), b"abc");
        assert!(read_bounded(&b"abcd"[..], 3).is_err());
        let digest = hex(&Sha256::digest(b"abc"));
        assert_eq!(
            digest,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let mut out = Vec::new();
        copy_verified(&b"abc"[..], &mut out, 3, &digest).unwrap();
        assert_eq!(out, b"abc");
        assert!(copy_verified(&b"abcd"[..], Vec::new(), 3, &digest).is_err());
        assert!(copy_verified(&b"ab"[..], Vec::new(), 3, &digest).is_err());
        assert!(copy_verified(&b"abc"[..], Vec::new(), 3, &"0".repeat(64)).is_err());
        assert!(copy_verified(&b"abc"[..], Vec::new(), MAX_PACKAGE_BYTES + 1, &digest).is_err());
        assert!(parse_release(
            &vec![b' '; MAX_RELEASE_BYTES as usize + 1],
            "0.8.2",
            "x64",
            UpdateKind::Msi
        )
        .is_err());
    }

    #[test]
    fn default_is_offline_and_unprepared_install_never_starts_a_process() {
        let mut updater = Updater::default();
        assert!(matches!(updater.phase, Phase::Idle));
        assert!(!updater.poll());
        assert!(updater.launch_prepared().is_err());
        let (tx, rx) = mpsc::channel();
        updater.phase = Phase::Checking;
        updater.rx = Some(rx);
        drop(tx);
        assert!(updater.poll());
        assert!(matches!(updater.phase, Phase::Failed(_)));
    }

    #[cfg(windows)]
    #[test]
    fn helper_paths_keep_unicode_and_unc_without_device_prefixes() {
        assert_eq!(
            ordinary_path(Path::new(r"\\?\C:\Users\Émanuel\SuniPDF")),
            Path::new(r"C:\Users\Émanuel\SuniPDF")
        );
        assert_eq!(
            ordinary_path(Path::new(r"\\?\UNC\server\share\SuniPDF")),
            Path::new(r"\\server\share\SuniPDF")
        );
        assert_eq!(
            ordinary_path(Path::new(r"C:\SuniPDF")),
            Path::new(r"C:\SuniPDF")
        );
        assert_eq!(
            ordinary_path(Path::new(r"\\.\device")),
            Path::new(r"\\.\device")
        );
    }

    #[test]
    #[ignore = "Requires GitHub network access; downloads a release without installing it."]
    fn live_release_download_verifies_sha256() {
        let mut metadata = https_get(
            LATEST_URL,
            "application/vnd.github+json",
            Duration::from_secs(20),
        )
        .unwrap();
        let body = read_bounded(metadata.body_mut().as_reader(), MAX_RELEASE_BYTES).unwrap();
        let release = parse_release(&body, "0.0.0", "x64", UpdateKind::Portable)
            .unwrap()
            .expect("a published stable x64 portable release");
        let mut response = https_get(
            &release.asset_url,
            "application/octet-stream",
            Duration::from_secs(300),
        )
        .unwrap();
        let mut file = tempfile::tempfile().unwrap();
        copy_verified(
            response.body_mut().as_reader(),
            &mut file,
            release.size,
            &release.sha256,
        )
        .unwrap();
        assert_eq!(file.metadata().unwrap().len(), release.size);
        println!(
            "Verified {} ({} bytes; SHA256 {})",
            release.asset_name, release.size, release.sha256
        );
    }
}
