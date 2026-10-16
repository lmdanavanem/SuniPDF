//! Independent structural checks of the repository's synthetic fixtures and CLI output.
//! Run explicitly with SUNIPDF_QPDF set by scripts/fetch-qpdf.ps1; no customer documents are used.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

#[path = "../../../crates/sunipdf-core/tests/support/certificate_identity.rs"]
mod test_identity;

#[path = "../../../crates/sunipdf-core/tests/fixtures/destinations.rs"]
mod destinations_fixture;

#[path = "../../../crates/sunipdf-core/tests/fixtures/radio_exports.rs"]
mod radio_exports;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/sunipdf-core/tests/fixtures")
        .join(name)
}

fn write_rgb_seed(path: &Path) {
    use lopdf::{dictionary, Document, Stream};
    let mut pdf = Document::with_version("1.7");
    let pages = pdf.new_object_id();
    let mut pixels = Vec::with_capacity(512 * 512 * 3);
    for y in 0..512 {
        for x in 0..512 {
            pixels.extend([
                ((x * 37 + y * 23) % 256) as u8,
                ((x * 17 + y * 53) % 256) as u8,
                ((x * 11 + y * 41) % 256) as u8,
            ]);
        }
    }
    // PDFium's PNG importer adds an SMask; this seed explicitly exercises unmasked RGB.
    let image = pdf.add_object(Stream::new(
        dictionary! {
            "Type"=>"XObject", "Subtype"=>"Image", "Width"=>512, "Height"=>512,
            "BitsPerComponent"=>8, "ColorSpace"=>"DeviceRGB"
        },
        pixels,
    ));
    let content = pdf.add_object(Stream::new(
        dictionary! {},
        b"q 400 0 0 400 0 0 cm /I Do Q".to_vec(),
    ));
    let page = pdf.add_object(dictionary! {
        "Type"=>"Page", "Parent"=>pages, "MediaBox"=>vec![0.into(),0.into(),400.into(),400.into()],
        "Resources"=>dictionary! {"XObject"=>dictionary! {"I"=>image}}, "Contents"=>content
    });
    pdf.objects.insert(
        pages,
        dictionary! {"Type"=>"Pages", "Kids"=>vec![page.into()], "Count"=>1}.into(),
    );
    let catalog = pdf.add_object(dictionary! {"Type"=>"Catalog", "Pages"=>pages});
    pdf.trailer.set("Root", catalog);
    pdf.save(path).unwrap();
}

fn bounded(mut command: Command, input: Option<&[u8]>) -> Output {
    let mut child = command
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("QA subprocess must start");
    if let Some(input) = input {
        child.stdin.take().unwrap().write_all(input).unwrap();
    }
    let start = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if start.elapsed() >= Duration::from_secs(180) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("QA subprocess exceeded its 180-second limit");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    child.wait_with_output().unwrap()
}

fn cli(args: &[&str], paths: &[&Path]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
    command.args(args);
    for path in paths {
        command.arg(path);
    }
    command
}

fn success(command: Command) -> serde_json::Value {
    let output = bounded(command, None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn check(qpdf: &Path, file: &Path, password: Option<&str>) {
    let mut command = Command::new(qpdf);
    if let Some(password) = password {
        command.arg(format!("--password={password}"));
    }
    command.arg("--check").arg(file);
    let output = bounded(command, None);
    // Exit 3 means structural warnings; it must fail this gate as well as exit 2.
    assert!(
        output.status.success(),
        "qpdf rejected {} ({}):\n{}\n{}",
        file.display(),
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    println!("qpdf OK: {}", file.file_name().unwrap().to_string_lossy());
}

#[test]
#[ignore = "requires the pinned official QPDF QA runtime; executed separately in Windows CI"]
fn synthetic_corpus_and_generated_outputs_pass_independent_structural_checks() {
    let qpdf =
        PathBuf::from(std::env::var_os("SUNIPDF_QPDF").expect("set SUNIPDF_QPDF to qpdf.exe"));
    let temp = tempfile::tempdir().unwrap();
    for name in [
        "two-pages.pdf",
        "boxes.pdf",
        "annotated.pdf",
        "acroform.pdf",
        "acroform-noap.pdf",
        "acroform-links.pdf",
        "acroform-xfa.pdf",
        "redact.pdf",
        "redact-incremental.pdf",
        "many-pages.pdf",
        "page-labels.pdf",
    ] {
        check(&qpdf, &fixture(name), None);
    }
    check(&qpdf, &fixture("encrypted-rc4.pdf"), Some("secret"));
    let destinations = temp.path().join("destinations.pdf");
    std::fs::write(&destinations, destinations_fixture::document()).unwrap();
    check(&qpdf, &destinations, None);
    let destinations_saved = temp.path().join("destinations-rewritten.pdf");
    let mut command = cli(&["set-metadata"], &[&destinations]);
    command
        .args(["--title", "Destination round trip", "--out"])
        .arg(&destinations_saved);
    success(command);
    check(&qpdf, &destinations_saved, None);
    let labelled = temp.path().join("authored-page-labels.pdf");
    let mut command = cli(&["set-page-labels"], &[&fixture("page-labels.pdf")]);
    command
        .args([
            "--from",
            "2",
            "--to",
            "4",
            "--style",
            "roman-lower",
            "--start",
            "9",
            "--prefix",
            "É-",
            "--out",
        ])
        .arg(&labelled);
    assert_eq!(success(command)["count"], 3);
    check(&qpdf, &labelled, None);
    let info = success(cli(&["info"], &[&labelled]));
    assert_eq!(info["pages"][1]["label"], "É-ix");
    assert_eq!(info["pages"][3]["label"], "É-xi");
    let reset_labels = temp.path().join("reset-page-labels.pdf");
    let mut command = cli(&["reset-page-labels"], &[&labelled]);
    command.arg("--out").arg(&reset_labels);
    assert_eq!(success(command)["count"], 9);
    check(&qpdf, &reset_labels, None);
    let info = success(cli(&["info"], &[&reset_labels]));
    assert!(info["pages"]
        .as_array()
        .unwrap()
        .iter()
        .all(|page| page["label"].is_null()));
    let original = fixture("two-pages.pdf");
    let original_bytes = std::fs::read(&original).unwrap();
    let signature_image = temp.path().join("signature.png");
    image::RgbaImage::from_pixel(80, 40, image::Rgba([20, 20, 20, 255]))
        .save(&signature_image)
        .unwrap();
    let marked = temp.path().join("signature-pages.pdf");
    let mut command = cli(&["add-image"], &[&fixture("boxes.pdf")]);
    command
        .arg("--image")
        .arg(&signature_image)
        .args(["--pages", "all", "--x", "100", "--y", "100", "--out"])
        .arg(&marked);
    let report = success(command);
    assert_eq!(report["pages"], serde_json::json!([1, 2, 3, 4]));
    assert_eq!(report["count"], 4);
    assert_eq!(report["digital_signature"], false);
    check(&qpdf, &marked, None);
    let edited = temp.path().join("edited.pdf");
    let mut command = cli(&["rotate"], &[&original]);
    command.args(["--degrees", "90", "--out"]).arg(&edited);
    success(command);
    check(&qpdf, &edited, None);
    for (name, aes128) in [("aes256.pdf", false), ("aes128.pdf", true)] {
        let encrypted = temp.path().join(name);
        let mut command = cli(&["protect"], &[&edited]);
        command
            .args([
                "--user-password",
                "package-user",
                "--owner-password",
                "package-owner",
                "--no-copy",
                "--no-print",
                "--out",
            ])
            .arg(&encrypted);
        if aes128 {
            command.arg("--aes128");
        }
        success(command);
        check(&qpdf, &encrypted, Some("package-user"));
        let opened = success(cli(
            &["properties", "--password", "package-user"],
            &[&encrypted],
        ));
        assert!(opened.is_object());
        assert_eq!(opened["permissions"]["copy"], false);
        assert_eq!(opened["permissions"]["print"], false);
        let encrypted_bytes = std::fs::read(&encrypted).unwrap();
        let labelled = temp.path().join(format!("{name}-authored-labels.pdf"));
        let mut command = cli(&["set-page-labels"], &[&encrypted]);
        command
            .args([
                "--password",
                "package-owner",
                "--style",
                "roman-lower",
                "--prefix",
                "É-",
                "--out",
            ])
            .arg(&labelled);
        assert_eq!(success(command)["count"], 2);
        for password in ["package-user", "package-owner"] {
            check(&qpdf, &labelled, Some(password));
            let info = success(cli(&["info", "--password", password], &[&labelled]));
            assert_eq!(info["pages"][0]["label"], "É-i");
            assert_eq!(info["pages"][1]["label"], "É-ii");
            let properties = success(cli(&["properties", "--password", password], &[&labelled]));
            assert_eq!(properties["permissions"], opened["permissions"]);
        }
        let reset_labels = temp.path().join(format!("{name}-reset-labels.pdf"));
        let mut command = cli(&["reset-page-labels"], &[&labelled]);
        command
            .args(["--password", "package-user", "--out"])
            .arg(&reset_labels);
        assert_eq!(success(command)["count"], 2);
        for password in ["package-user", "package-owner"] {
            check(&qpdf, &reset_labels, Some(password));
            let info = success(cli(&["info", "--password", password], &[&reset_labels]));
            assert!(info["pages"]
                .as_array()
                .unwrap()
                .iter()
                .all(|page| page["label"].is_null()));
            let text = bounded(
                cli(&["text", "--password", password], &[&reset_labels]),
                None,
            );
            assert!(text.status.success());
            assert!(String::from_utf8_lossy(&text.stdout).contains("SuniPDF page 1"));
            assert!(String::from_utf8_lossy(&text.stdout).contains("SuniPDF page 2"));
        }
        assert_eq!(std::fs::read(&encrypted).unwrap(), encrypted_bytes);
        let marked = temp.path().join(format!("{name}-signature-pages.pdf"));
        let mut command = cli(&["add-image"], &[&encrypted]);
        command
            .arg("--image")
            .arg(&signature_image)
            .args([
                "--password",
                "package-owner",
                "--pages",
                "all",
                "--x",
                "100",
                "--y",
                "100",
                "--out",
            ])
            .arg(&marked);
        assert_eq!(success(command)["count"], 2);
        check(&qpdf, &marked, Some("package-user"));
        check(&qpdf, &marked, Some("package-owner"));
        let extracted = temp.path().join(format!("{name}-extract.pdf"));
        let mut command = cli(&["extract"], &[&encrypted]);
        command
            .args(["--password", "package-owner", "--pages", "2,1", "--out"])
            .arg(&extracted);
        let report = success(command);
        assert_eq!(report["encryption_removed"], false);
        assert_eq!(report["encrypted"], true);
        check(&qpdf, &extracted, Some("package-user"));
        check(&qpdf, &extracted, Some("package-owner"));
        let mut command = cli(&["split"], &[&encrypted]);
        command
            .args(["--password", "package-owner", "--every", "1", "--out-dir"])
            .arg(temp.path().join(format!("{name}-parts")));
        let report = success(command);
        assert_eq!(report["encryption_removed"], false);
        assert_eq!(report["encrypted"], true);
        for part in report["files"].as_array().unwrap() {
            check(
                &qpdf,
                Path::new(part.as_str().unwrap()),
                Some("package-user"),
            );
        }
        let metadata = temp.path().join(format!("{name}-owner-metadata.pdf"));
        let mut command = cli(&["set-metadata"], &[&encrypted]);
        command
            .args([
                "--password",
                "package-owner",
                "--title",
                "Owner stream regression",
                "--out",
            ])
            .arg(&metadata);
        success(command);
        check(&qpdf, &metadata, Some("package-user"));
        let text = bounded(
            cli(&["text", "--password", "package-user"], &[&metadata]),
            None,
        );
        assert!(text.status.success());
        assert!(String::from_utf8_lossy(&text.stdout).contains("SuniPDF page 1"));
        assert!(String::from_utf8_lossy(&text.stdout).contains("SuniPDF page 2"));
    }
    let field = temp.path().join("field.pdf");
    let mut command = cli(&["add-form-field"], &[&original]);
    command
        .args([
            "--kind",
            "text",
            "--name",
            "FullName",
            "--rect",
            "40,40,220,65",
            "--out",
        ])
        .arg(&field);
    success(command);
    check(&qpdf, &field, None);
    let filled = temp.path().join("filled.pdf");
    let mut command = cli(&["fill-form"], &[&field]);
    command
        .args(["--set", "FullName=Alice", "--out"])
        .arg(&filled);
    success(command);
    check(&qpdf, &filled, None);
    let flat = temp.path().join("flattened.pdf");
    let mut command = cli(&["flatten"], &[&filled]);
    command.arg("--out").arg(&flat);
    success(command);
    check(&qpdf, &flat, None);

    let mut choices = filled.clone();
    for (kind, name, rect, multi_select, editable) in [
        ("radio", "Contact", "40,100,220,180", false, false),
        ("combo", "Country", "40,220,220,250", false, false),
        ("combo", "OtherCountry", "40,390,220,420", false, true),
        ("list", "Languages", "40,290,220,370", true, false),
    ] {
        let output = temp.path().join(format!("created-{kind}-{name}.pdf"));
        let mut command = cli(&["add-form-field"], &[&choices]);
        command.args([
            "--kind",
            kind,
            "--name",
            name,
            "--rect",
            rect,
            "--option",
            "one=First choice",
            "--option",
            "two=Second choice",
        ]);
        if multi_select {
            command.arg("--multi-select");
        }
        if editable {
            command.arg("--editable");
        }
        command.arg("--out").arg(&output);
        success(command);
        check(&qpdf, &output, None);
        choices = output;
    }
    let choices_filled = temp.path().join("choices-filled.pdf");
    let mut command = cli(&["fill-form"], &[&choices]);
    command
        .args([
            "--set",
            "Contact=two",
            "--set",
            "Country=one",
            "--set",
            "OtherCountry=Suisse romande – été",
            "--set",
            "Languages=one;two",
            "--out",
        ])
        .arg(&choices_filled);
    success(command);
    check(&qpdf, &choices_filled, None);
    let values = success(cli(&["form-fields", "--values-only"], &[&choices_filled]));
    assert_eq!(values["Contact"], "two");
    assert_eq!(values["Country"], "one");
    assert_eq!(values["OtherCountry"], "Suisse romande – été");
    assert_eq!(values["Languages"], serde_json::json!(["one", "two"]));
    let choices_flat = temp.path().join("choices-flattened.pdf");
    let mut command = cli(&["flatten"], &[&choices_filled]);
    command.arg("--out").arg(&choices_flat);
    success(command);
    check(&qpdf, &choices_flat, None);
    assert_eq!(success(cli(&["form-fields"], &[&choices_flat]))["count"], 0);

    let radio = temp.path().join("radio-exports.pdf");
    std::fs::write(&radio, radio_exports::source(true)).unwrap();
    check(&qpdf, &radio, None);
    let radio_filled = temp.path().join("radio-exports-filled.pdf");
    let selection = format!(
        "{}={}",
        radio_exports::FIELD_NAME,
        radio_exports::EXPORTS[0]
    );
    let mut command = cli(&["fill-form"], &[&radio]);
    command
        .args(["--set", &selection, "--out"])
        .arg(&radio_filled);
    success(command);
    check(&qpdf, &radio_filled, None);
    assert_eq!(
        success(cli(&["form-fields", "--values-only"], &[&radio_filled]))
            [radio_exports::FIELD_NAME],
        radio_exports::EXPORTS[0]
    );
    let radio_flat = temp.path().join("radio-exports-flattened.pdf");
    let mut command = cli(&["flatten"], &[&radio_filled]);
    command.arg("--out").arg(&radio_flat);
    success(command);
    check(&qpdf, &radio_flat, None);
    assert_eq!(success(cli(&["form-fields"], &[&radio_flat]))["count"], 0);

    let group_rotated = temp.path().join("group-rotated.pdf");
    let mut command = cli(&["rotate-objects"], &[&original]);
    command
        .args(["--objects", "0,1", "--degrees", "90", "--out"])
        .arg(&group_rotated);
    success(command);
    check(&qpdf, &group_rotated, None);
    let group_resized = temp.path().join("group-resized.pdf");
    let mut command = cli(&["resize-objects"], &[&group_rotated]);
    command
        .args(["--objects", "0,1", "--rect", "100,100,400,300", "--out"])
        .arg(&group_resized);
    success(command);
    check(&qpdf, &group_resized, None);
    let redacted = temp.path().join("redacted.pdf");
    let mut command = cli(&["redact"], &[&fixture("redact.pdf")]);
    command
        .args(["--text", "SECRET-ALPHA-7731", "--out"])
        .arg(&redacted);
    success(command);
    check(&qpdf, &redacted, None);
    let search = success(cli(
        &["search"],
        &[&redacted, Path::new("SECRET-ALPHA-7731")],
    ));
    assert_eq!(search["count"], 0);
    let revision = temp.path().join("redacted-revision.pdf");
    let mut command = cli(&["redact"], &[&fixture("redact-incremental.pdf")]);
    command
        .args(["--text", "Current revision", "--out"])
        .arg(&revision);
    success(command);
    check(&qpdf, &revision, None);
    assert!(!std::fs::read(&revision)
        .unwrap()
        .windows(16)
        .any(|bytes| bytes == b"OLDREV-SECRET-01"));

    let png = temp.path().join("scan.png");
    let mut command = cli(&["render"], &[&original]);
    command
        .args(["--page", "1", "--scale", "2", "--out"])
        .arg(&png);
    success(command);
    let scan = temp.path().join("scan.pdf");
    let mut command = cli(&["images2pdf"], &[&png]);
    command.args(["--dpi", "144", "--out"]).arg(&scan);
    success(command);
    check(&qpdf, &scan, None);
    let compression_seed = temp.path().join("compression-seed.pdf");
    write_rgb_seed(&compression_seed);
    check(&qpdf, &compression_seed, None);
    let seed_bytes = std::fs::read(&compression_seed).unwrap();
    let compressed = temp.path().join("compressed.pdf");
    let mut command = cli(&["compress-images"], &[&compression_seed]);
    command
        .args(["--max-image-side", "128", "--jpeg-quality", "50", "--out"])
        .arg(&compressed);
    let report = success(command);
    assert!(report["images_changed"].as_u64().unwrap() >= 1);
    assert!(report["after"].as_u64().unwrap() < report["before"].as_u64().unwrap());
    check(&qpdf, &compressed, None);
    assert_eq!(std::fs::read(&compression_seed).unwrap(), seed_bytes);
    let ocr = temp.path().join("ocr.pdf");
    let mut command = cli(&["ocr"], &[&scan]);
    command.args(["--language", "eng+fra", "--out"]).arg(&ocr);
    let report = success(command);
    assert_eq!(report["recognized_pages"], 1);
    assert!(report["words"].as_u64().unwrap() >= 3);
    check(&qpdf, &ocr, None);

    let pfx = temp.path().join("identity.pfx");
    std::fs::write(&pfx, test_identity::identity()).unwrap();
    let signed = temp.path().join("signed.pdf");
    let mut command = cli(&["sign"], &[&original]);
    command
        .arg("--certificate")
        .arg(&pfx)
        .arg("--certificate-password-stdin")
        .arg("--out")
        .arg(&signed);
    let output = bounded(command, Some(b"test-only\n"));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    check(&qpdf, &signed, None);
    let report = success(cli(&["signatures"], &[&signed]));
    assert_eq!(report["signatures"][0]["status"], "valid");

    for (name, bytes) in [
        ("empty.pdf", b"".as_slice()),
        ("not-pdf.pdf", b"This is not a PDF.".as_slice()),
        (
            "truncated.pdf",
            b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog".as_slice(),
        ),
    ] {
        let malformed = temp.path().join(name);
        std::fs::write(&malformed, bytes).unwrap();
        let mut command = Command::new(&qpdf);
        command.arg("--check").arg(&malformed);
        assert_eq!(
            bounded(command, None).status.code(),
            Some(2),
            "qpdf should reject {name}"
        );
        let out = temp.path().join(format!("{name}.output.pdf"));
        let mut command = cli(&["rotate"], &[&malformed]);
        command.args(["--degrees", "90", "--out"]).arg(&out);
        let output = bounded(command, None);
        assert!(!output.status.success(), "CLI accepted {name}");
        let error: serde_json::Value =
            serde_json::from_slice(&output.stderr).expect("typed CLI error, not a process crash");
        assert!(error["error"].is_string());
        assert!(!out.exists(), "failed operation left output for {name}");
        assert_eq!(std::fs::read(&malformed).unwrap(), bytes);
    }
    assert_eq!(std::fs::read(&original).unwrap(), original_bytes);
}
