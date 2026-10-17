use super::*;

#[test]
fn profiles_versions_urls_and_local_path_guards_are_explicit() {
    assert_eq!(Profile::from_id("print"), Some(Profile::Print));
    assert!(Profile::from_id("archival").is_none());
    for version in ["LibreOffice 26.2.6.2 abc", "LibreOffice 26.8.1.1"] {
        assert!(supported_version(version));
    }
    for version in [
        "LibreOffice 7.6.7.2",
        "LibreOffice 26.20.1",
        "untrusted 26.8.1",
        "",
    ] {
        assert!(!supported_version(version));
    }
    assert!(Profile::Standard.filter_options().contains("150"));
    assert!(Profile::Print
        .filter_options()
        .contains("UseLosslessCompression"));
    assert_eq!(
        file_url(Path::new("C:\\A B\\é#profile")).unwrap(),
        "file:///C:/A%20B/%C3%A9%23profile"
    );
    assert!(validate_local_output(Path::new("relative.docx")).is_err());
    assert!(validate_local_path(Path::new("C:/safe/../outside")).is_err());
    #[cfg(windows)]
    {
        assert!(validate_local_path(Path::new(r"\\server\share\file.docx")).is_err());
        assert!(validate_local_path(Path::new(r"C:\file.docx:stream")).is_err());
    }
    let xml = profile_config();
    let reader = xml::reader::EventReader::new(xml.as_bytes());
    for event in reader {
        event.unwrap();
    }
    for property in [
        "DisableMacrosExecution",
        "DisableActiveContent",
        "MacroSecurityLevel",
        "ooInetHTTPSProxyPort",
    ] {
        assert!(xml.contains(property));
    }
}

#[cfg(windows)]
fn package(dir: &Path, xml: &str, extras: &[(&str, &[u8])]) -> PathBuf {
    let mut package = crate::export_office::archive::Package::default();
    package.part("[Content_Types].xml", b"<Types/>").unwrap();
    package.part("word/document.xml", xml.as_bytes()).unwrap();
    for (name, bytes) in extras {
        package.part(name, bytes).unwrap();
    }
    let path = dir.join("input.docx");
    fs::write(&path, package.finish().unwrap()).unwrap();
    path
}

#[cfg(windows)]
#[test]
fn office_archive_guards_refuse_active_external_malformed_and_disguised_parts() {
    let dir = tempfile::tempdir().unwrap();
    let cancel = AtomicBool::new(false);
    let safe = package(
        dir.path(),
        "<document><text>Local &amp; simple</text></document>",
        &[],
    );
    validate_package(&safe, "docx", &cancel, dir.path()).unwrap();
    for xml in [
        "<!DOCTYPE d [<!ENTITY x SYSTEM 'file:///C:/secret'>]><d>&x;</d>",
        "<d><instrText>DDE</instrText></d>",
        "<d><f>WEBSERVICE()</f></d>",
        "<d xmlns:x='http://www.w3.org/1999/xlink' x:href='https://example.test/x'/>",
        "<d><broken></d>",
    ] {
        let path = package(dir.path(), xml, &[]);
        assert!(
            validate_package(&path, "docx", &cancel, dir.path()).is_err(),
            "accepted {xml}"
        );
    }
    for (name,bytes) in [("word/vbaProject.bin",b"macro".as_slice()),("word/embeddings/object.xml",b"<object/>".as_slice()),("word/media/image.png",b"<svg onload='fetch()'/>".as_slice()),("word/_rels/document.xml.rels",b"<Relationships><Relationship TargetMode='External' Target='https://example.test/file' Type='image'/></Relationships>".as_slice()),("word/_rels/document.xml.rels",b"<Relationships><Relationship Target='../../../outside' Type='image'/></Relationships>".as_slice())] {
        let path=package(dir.path(),"<document/>",&[(name,bytes)]); assert!(validate_package(&path,"docx",&cancel,dir.path()).is_err(),"accepted {name}");
    }
    let path = package(dir.path(), "<document/>", &[]);
    assert!(validate_package(&path, "xlsx", &cancel, dir.path()).is_err());
}

#[cfg(windows)]
#[test]
fn local_process_is_bounded_and_pre_cancel_does_not_launch() {
    let dir = tempfile::tempdir().unwrap();
    let cancel = AtomicBool::new(true);
    let mut command = Command::new(system_tool("WindowsPowerShell/v1.0/powershell.exe").unwrap());
    command.args(["-NoProfile", "-Command", "Start-Sleep -Seconds 30"]);
    assert!(matches!(
        run(
            command,
            &cancel,
            Duration::from_millis(100),
            &dir.path().join("cancel.log"),
            None
        ),
        Err(EngineError::Cancelled)
    ));
    assert!(!dir.path().join("cancel.log").exists());
    cancel.store(false, Ordering::Relaxed);
    let mut command = Command::new(system_tool("WindowsPowerShell/v1.0/powershell.exe").unwrap());
    command.args(["-NoProfile", "-Command", "Start-Sleep -Seconds 30"]);
    let start = Instant::now();
    assert!(run(
        command,
        &cancel,
        Duration::from_millis(100),
        &dir.path().join("timeout.log"),
        None
    )
    .is_err());
    assert!(start.elapsed() < Duration::from_secs(10));
}
