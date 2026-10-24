//! Ubuntu package-manager handoff. The fixed installed helper independently authenticates
//! release metadata and stages the package before invoking the system package manager.

use crate::updater::{PreparedUpdate, UpdateKind};
use std::ffi::OsString;
use std::fs;
use std::io::Read;
#[cfg(target_os = "linux")]
use std::path::Path;
use std::process::{Command, Stdio};

const HELPER: &str = "/usr/lib/sunipdf/install-update";
#[cfg(target_os = "linux")]
const EXECUTABLE: &str = "/usr/lib/sunipdf/sunipdf";
#[cfg(target_os = "linux")]
const LAUNCHER: &str = "/usr/bin/sunipdf";
const PKEXEC: &str = "/usr/bin/pkexec";
const MAX_CAPTURE: usize = 16 * 1024;

pub(crate) fn require_system_installation() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let current = fs::canonicalize(std::env::current_exe().map_err(|e| e.to_string())?)
            .map_err(|_| {
                "The running application was replaced. Restart SuniPDF before updating."
            })?;
        if current != Path::new(EXECUTABLE) {
            return Err("Direct updates require the system-installed Ubuntu package. Install the current official .deb once, then retry.".into());
        }
        require_installed_files()
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err("Ubuntu package installation is available only on Linux.".into())
    }
}

#[cfg(target_os = "linux")]
fn require_installed_files() -> Result<(), String> {
    use std::os::unix::fs::MetadataExt;
    for path in ["/", "/usr", "/usr/bin", "/usr/lib", "/usr/lib/sunipdf"] {
        let metadata = fs::symlink_metadata(path)
            .map_err(|_| "The installed application directory is unavailable.")?;
        if !metadata.is_dir() || metadata.uid() != 0 || metadata.mode() & 0o022 != 0 {
            return Err(
                "The installed application directory is not protected by the system.".into(),
            );
        }
    }
    for path in [HELPER, EXECUTABLE, LAUNCHER, PKEXEC] {
        let metadata = fs::symlink_metadata(path)
            .map_err(|_| "The Ubuntu update installer is missing. Reinstall the current official .deb once, then retry.")?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.uid() != 0
            || metadata.mode() & 0o022 != 0
            || metadata.mode() & 0o111 == 0
        {
            return Err("The Ubuntu update installer is not a protected system executable.".into());
        }
    }
    Ok(())
}

fn arguments(prepared: &PreparedUpdate) -> Result<Vec<OsString>, String> {
    let release = &prepared.release;
    let version =
        semver::Version::parse(&release.version).map_err(|_| "The update version is invalid.")?;
    if release.kind != UpdateKind::Deb
        || release.arch != "amd64"
        || !version.pre.is_empty()
        || !version.build.is_empty()
        || version.to_string() != release.version
        || release.size == 0
        || release.size > 256 * 1024 * 1024
        || release.sha256.len() != 64
        || !release.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        || release.asset_name != format!("SuniPDF-{}-linux-amd64.deb", release.version)
    {
        return Err("The prepared Ubuntu update has an invalid identity.".into());
    }
    let workspace = prepared
        .workspace
        .as_ref()
        .ok_or("The verified update folder is unavailable.")?;
    if !prepared.package.is_absolute()
        || prepared.package != workspace.path().join(&release.asset_name)
    {
        return Err("The verified update package is outside its private folder.".into());
    }
    let metadata = fs::symlink_metadata(&prepared.package)
        .map_err(|_| "The verified update package is unavailable.")?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() != release.size {
        return Err("The verified update package changed before installation.".into());
    }
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::MetadataExt;
        let directory = fs::symlink_metadata(workspace.path()).map_err(|e| e.to_string())?;
        if !directory.is_dir()
            || directory.mode() & 0o077 != 0
            || metadata.uid() != directory.uid()
            || metadata.nlink() != 1
            || metadata.mode() & 0o777 != 0o600
        {
            return Err("The verified update is not stored in a private user folder.".into());
        }
    }
    Ok(vec![
        HELPER.into(),
        "--package".into(),
        prepared.package.as_os_str().to_owned(),
        "--sha256".into(),
        release.sha256.clone().into(),
        "--version".into(),
        release.version.clone().into(),
        "--size".into(),
        release.size.to_string().into(),
    ])
}

#[derive(Debug)]
struct Captured {
    bytes: Vec<u8>,
    truncated: bool,
}

fn drain(mut reader: impl Read) -> std::io::Result<Captured> {
    let mut captured = Captured {
        bytes: Vec::new(),
        truncated: false,
    };
    let mut buffer = [0; 4096];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            return Ok(captured);
        }
        let keep = count.min(MAX_CAPTURE.saturating_sub(captured.bytes.len()));
        captured.bytes.extend_from_slice(&buffer[..keep]);
        captured.truncated |= keep < count;
    }
}

fn installation_result(
    code: Option<i32>,
    stdout: Captured,
    stderr: Captured,
    version: &str,
) -> Result<(), String> {
    if code == Some(126) {
        return Err(
            "Linux authorization was cancelled. Nothing was installed; you can retry.".into(),
        );
    }
    if code == Some(127) {
        return Err("Linux did not authorize the installer. Complete the system authorization dialog or check that an authentication agent is running, then retry.".into());
    }
    if code == Some(0) && !stdout.truncated {
        let result = serde_json::from_slice::<serde_json::Value>(&stdout.bytes);
        if result.is_ok_and(|value| {
            value.get("status").and_then(|v| v.as_str()) == Some("installed")
                && value.get("version").and_then(|v| v.as_str()) == Some(version)
        }) {
            return Ok(());
        }
        return Err("The installer returned an unconfirmed result. Keep this session open and check the installed version before restarting.".into());
    }
    let detail = if !stderr.truncated {
        serde_json::from_slice::<serde_json::Value>(&stderr.bytes)
            .ok()
            .and_then(|value| {
                value
                    .get("error")
                    .and_then(|v| v.as_str())
                    .map(str::to_owned)
            })
            .map(|value| {
                value
                    .chars()
                    .filter(|c| !c.is_control())
                    .take(1024)
                    .collect::<String>()
            })
    } else {
        None
    };
    Err(detail.filter(|value| !value.is_empty())
        .unwrap_or_else(|| "The Ubuntu update did not complete. Check the package manager and retry; your documents remain open.".into()))
}

pub(crate) fn install(prepared: &PreparedUpdate) -> Result<(), String> {
    require_system_installation()?;
    let args = arguments(prepared)?;
    let mut command = Command::new(PKEXEC);
    command
        .args(args)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LANG", "C.UTF-8");
    // Retain only the normal user's display/session coordinates for the authentication agent.
    for key in [
        "DISPLAY",
        "WAYLAND_DISPLAY",
        "XAUTHORITY",
        "XDG_RUNTIME_DIR",
        "DBUS_SESSION_BUS_ADDRESS",
    ] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not start Linux authorization: {e}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or("The installer output is unavailable.")?;
    let stderr = child
        .stderr
        .take()
        .ok_or("The installer diagnostics are unavailable.")?;
    let stdout = std::thread::spawn(move || drain(stdout));
    let stderr = std::thread::spawn(move || drain(stderr));
    // Never interrupt an authenticated dpkg transaction. The UI keeps the private package
    // alive and offers an elapsed-time notice; network/query stages have their own bounds.
    let status = child
        .wait()
        .map_err(|e| format!("Could not monitor the package manager: {e}"))?;
    let stdout = stdout
        .join()
        .map_err(|_| "The installer output reader stopped.")?
        .map_err(|e| e.to_string())?;
    let stderr = stderr
        .join()
        .map_err(|_| "The installer diagnostic reader stopped.")?
        .map_err(|e| e.to_string())?;
    installation_result(status.code(), stdout, stderr, &prepared.release.version)
}

#[cfg_attr(test, allow(dead_code))]
pub(crate) fn restart() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        require_installed_files()?;
        Command::new(LAUNCHER)
            .spawn()
            .map_err(|e| format!("Could not restart SuniPDF: {e}"))?;
        Ok(())
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err("Ubuntu restart is available only on Linux.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn captured(value: &[u8]) -> Captured {
        Captured {
            bytes: value.to_vec(),
            truncated: false,
        }
    }

    #[test]
    fn package_handoff_uses_literal_arguments_and_rejects_changed_identity_or_path() {
        let mut prepared = crate::updater::test_prepared_deb();
        let original = prepared.package.clone();
        let args = arguments(&prepared).unwrap();
        assert_eq!(args.len(), 9);
        assert_eq!(args[0], OsString::from(HELPER));
        assert_eq!(args[2], original.as_os_str());
        assert_eq!(args[1], OsString::from("--package"));
        assert_eq!(args[5], OsString::from("--version"));
        assert_eq!(args[6], OsString::from("0.74.0"));
        assert!(original.to_string_lossy().contains("$ ;"));
        prepared.release.version = "0.74.0; touch /tmp/unexpected".into();
        assert!(arguments(&prepared).is_err());
        prepared.release.version = "0.74.0".into();
        prepared.release.sha256 = "z".repeat(64);
        assert!(arguments(&prepared).is_err());
        prepared.release.sha256 = "0".repeat(64);
        prepared.package = prepared
            .workspace
            .as_ref()
            .unwrap()
            .path()
            .join("../outside.deb");
        assert!(arguments(&prepared).is_err());
        prepared.package = original;
        fs::write(&prepared.package, b"changed size").unwrap();
        assert!(arguments(&prepared).is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn package_handoff_refuses_public_permissions_links_and_missing_installer_context() {
        use std::os::unix::fs::PermissionsExt;
        let prepared = crate::updater::test_prepared_deb();
        fs::set_permissions(&prepared.package, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(arguments(&prepared).is_err());
        fs::set_permissions(&prepared.package, fs::Permissions::from_mode(0o600)).unwrap();
        let link = prepared.workspace.as_ref().unwrap().path().join("hardlink");
        fs::hard_link(&prepared.package, &link).unwrap();
        assert!(arguments(&prepared).is_err());
        fs::remove_file(&link).unwrap();
        fs::remove_file(&prepared.package).unwrap();
        std::os::unix::fs::symlink("/dev/null", &prepared.package).unwrap();
        assert!(arguments(&prepared).is_err());
        assert!(
            require_system_installation().is_err(),
            "Unit-test binaries cannot install system updates"
        );
    }

    #[test]
    fn success_requires_exact_installed_version_and_cancellation_never_succeeds() {
        let good = br#"{"status":"installed","version":"0.74.0"}"#;
        assert!(installation_result(Some(0), captured(good), captured(b""), "0.74.0").is_ok());
        for (code, output, version) in [
            (Some(0), good.as_slice(), "0.75.0"),
            (Some(0), b"not json".as_slice(), "0.74.0"),
            (Some(1), good.as_slice(), "0.74.0"),
            (Some(126), good.as_slice(), "0.74.0"),
            (Some(127), good.as_slice(), "0.74.0"),
            (None, good.as_slice(), "0.74.0"),
        ] {
            assert!(installation_result(code, captured(output), captured(b""), version).is_err());
        }
        let mut oversized = captured(good);
        oversized.truncated = true;
        assert!(installation_result(Some(0), oversized, captured(b""), "0.74.0").is_err());
    }

    #[test]
    fn installer_output_is_drained_without_unbounded_memory_or_control_text() {
        let mut input = std::io::Cursor::new(vec![b'x'; MAX_CAPTURE * 5]);
        let result = drain(&mut input).unwrap();
        assert!(result.truncated);
        assert_eq!(result.bytes.len(), MAX_CAPTURE);
        assert_eq!(input.position(), (MAX_CAPTURE * 5) as u64);
        let error =
            serde_json::json!({"status":"failed","error":format!("\u{1b}{}", "x".repeat(2000))});
        let result = installation_result(
            Some(1),
            captured(b""),
            captured(&serde_json::to_vec(&error).unwrap()),
            "0.74.0",
        )
        .unwrap_err();
        assert_eq!(result.chars().count(), 1024);
        assert!(!result.chars().any(char::is_control));
    }
}
