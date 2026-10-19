pub fn identity() -> Vec<u8> {
    identity_with_password("test-only")
}

pub fn identity_with_password(password: &str) -> Vec<u8> {
    identity_with_subject(password, "/CN=SuniPDF Test Identity")
}

pub fn identity_with_subject(password: &str, subject: &str) -> Vec<u8> {
    use std::process::Command;
    let temp = tempfile::tempdir().unwrap();
    let executable = openssl();
    let key = temp.path().join("identity.key");
    let cert = temp.path().join("identity.crt");
    let pfx = temp.path().join("identity.pfx");
    let result = Command::new(&executable)
        .args([
            "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-subj", subject, "-days", "2",
            "-keyout",
        ])
        .arg(&key)
        .arg("-out")
        .arg(&cert)
        .output()
        .expect("OpenSSL from Git for Windows is required for certificate tests");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let result = Command::new(executable)
        .args(["pkcs12", "-export", "-inkey"])
        .arg(&key)
        .arg("-in")
        .arg(&cert)
        .arg("-out")
        .arg(&pfx)
        .arg("-passout")
        .arg(format!("pass:{password}"))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    std::fs::read(pfx).unwrap()
}

pub fn openssl() -> std::path::PathBuf {
    let installed = std::path::Path::new("C:/Program Files/Git/usr/bin/openssl.exe");
    if installed.exists() {
        installed.into()
    } else {
        "openssl".into()
    }
}
