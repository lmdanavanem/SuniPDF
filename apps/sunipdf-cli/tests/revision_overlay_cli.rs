use serde_json::Value;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use sunipdf_core::Protection;

fn seed(path: &Path, rotation: i64) {
    use lopdf::{dictionary, Document, Stream};
    let mut pdf = Document::with_version("1.7");
    let pages = pdf.new_object_id();
    let contents = pdf.add_object(Stream::new(
        dictionary! {},
        b"0 0 0 rg 60 80 45 70 re f\n".to_vec(),
    ));
    let page = pdf.add_object(dictionary! {
        "Type"=>"Page", "Parent"=>pages,
        "MediaBox"=>vec![0.into(),0.into(),400.into(),300.into()],
        "CropBox"=>vec![20.into(),30.into(),380.into(),270.into()],
        "Rotate"=>rotation, "Resources"=>dictionary! {}, "Contents"=>contents,
    });
    pdf.objects.insert(
        pages,
        dictionary! {"Type"=>"Pages","Kids"=>vec![page.into()],"Count"=>1}.into(),
    );
    let root = pdf.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
    pdf.trailer.set("Root", root);
    pdf.save(path).unwrap();
}

fn cli(base: &Path, revision: &Path, out: &Path, format: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
    command
        .arg("revision-overlay")
        .arg(base)
        .arg(revision)
        .args(["--format", format, "--max-side", "128", "--out"])
        .arg(out);
    command
}

fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}

fn failure(output: Output) -> Option<Value> {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    serde_json::from_slice(&output.stderr).ok()
}

fn piped(mut command: Command, bytes: &[u8]) -> Output {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    child.wait_with_output().unwrap()
}

fn png(path: &Path) -> image::RgbaImage {
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    image::load_from_memory(&bytes).unwrap().into_rgba8()
}

#[test]
fn exports_identity_overlay_with_crop_rotation_source_hashes_and_unchanged_inputs() {
    let directory = tempfile::tempdir().unwrap();
    for rotation in [0, 90, 180, 270] {
        let source = directory.path().join(format!("page-{rotation}.pdf"));
        seed(&source, rotation);
        let bytes = std::fs::read(&source).unwrap();
        let image_path = directory.path().join(format!("overlay-{rotation}.png"));
        let result = success(cli(&source, &source, &image_path, "png").output().unwrap());
        assert_eq!(result["originals_unchanged"], true);
        let report = &result["report"];
        assert_eq!(report["schema"], "sunipdf-revision-overlay");
        assert_eq!(report["version"], 1);
        assert_eq!(report["base"]["page"], 1);
        assert_eq!(report["revision"]["page"], 1);
        assert_eq!(report["base"]["rotation_degrees"], rotation);
        assert_eq!(
            report["base"]["crop_box_user_pt"],
            serde_json::json!([20.0, 30.0, 380.0, 270.0])
        );
        let (width, height) = if rotation % 180 == 0 {
            (360.0, 240.0)
        } else {
            (240.0, 360.0)
        };
        assert_eq!(report["base"]["display_width_pt"], width);
        assert_eq!(report["base"]["display_height_pt"], height);
        assert_eq!(report["coordinates"], "display_top_left_points");
        assert_eq!(report["colors"]["base"], "#ff00ff");
        assert_eq!(report["colors"]["revision"], "#00ffff");
        assert_eq!(
            report["base"]["source_sha256"],
            report["revision"]["source_sha256"]
        );
        let hash = report["base"]["source_sha256"].as_str().unwrap();
        assert_eq!(hash.len(), 64);
        assert!(hash.bytes().all(|byte| byte.is_ascii_hexdigit()));
        let image = png(&image_path);
        assert!(image.width() <= 128 && image.height() <= 128);
        assert_eq!(report["canvas"]["width"], image.width());
        assert_eq!(report["canvas"]["height"], image.height());
        assert!(image.pixels().any(|pixel| pixel.0[..3] != [255, 255, 255]));
        let manifest = directory.path().join(format!("overlay-{rotation}.json"));
        let exported = success(
            cli(&source, &source, &manifest, "json")
                .args([
                    "--expected-base-sha256",
                    hash,
                    "--expected-revision-sha256",
                    hash,
                ])
                .output()
                .unwrap(),
        );
        let read: Value = serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
        assert_eq!(read, exported["report"]);
        assert_eq!(read, *report);
        assert_eq!(std::fs::read(&source).unwrap(), bytes);
    }
}

#[test]
fn two_point_alignment_matches_explicit_clockwise_rotation_translation_and_scale() {
    let directory = tempfile::tempdir().unwrap();
    let base = directory.path().join("base.pdf");
    let revision = directory.path().join("revision.pdf");
    seed(&base, 90);
    seed(&revision, 90);
    for (case, base_points, revision_points, translation, rotation, scale) in [
        ("scaled", "30,40 130,40", "20,20 70,20", "-10,0", "0", "2"),
        (
            "clockwise",
            "80,40 80,140",
            "10,20 110,20",
            "100,30",
            "90",
            "1",
        ),
        (
            "half-turn",
            "160,160 60,160",
            "10,20 110,20",
            "170,180",
            "180",
            "1",
        ),
    ] {
        let anchors = directory.path().join(format!("{case}-anchors.png"));
        let manual = directory.path().join(format!("{case}-manual.png"));
        let inferred = success(
            cli(&base, &revision, &anchors, "png")
                .args([
                    "--base-points",
                    base_points,
                    "--revision-points",
                    revision_points,
                ])
                .output()
                .unwrap(),
        );
        let declared = success(
            cli(&base, &revision, &manual, "png")
                .args([
                    "--translate",
                    translation,
                    "--rotation",
                    rotation,
                    "--scale",
                    scale,
                ])
                .output()
                .unwrap(),
        );
        let actual = &inferred["report"]["transform"];
        let expected = &declared["report"]["transform"];
        for axis in [0, 1] {
            assert!(
                (actual["translation"][axis].as_f64().unwrap()
                    - expected["translation"][axis].as_f64().unwrap())
                .abs()
                    < 1e-8
            );
        }
        assert!(
            (actual["rotation_degrees"].as_f64().unwrap()
                - expected["rotation_degrees"].as_f64().unwrap())
            .abs()
                < 1e-8
        );
        assert!(
            (actual["scale"].as_f64().unwrap() - expected["scale"].as_f64().unwrap()).abs() < 1e-8
        );
        let a = png(&anchors);
        let b = png(&manual);
        assert_eq!(a.dimensions(), b.dimensions());
        // The same explicit similarity transform must produce the same sampled pixels.
        assert!(a
            .as_raw()
            .iter()
            .zip(b.as_raw())
            .all(|(a, b)| a.abs_diff(*b) <= 1));
        assert_eq!(inferred["report"]["alignment"]["mode"], "two_points");
        assert_eq!(declared["report"]["alignment"]["mode"], "manual");
    }
}

#[test]
fn invalid_alignment_stale_sources_and_output_collisions_never_create_or_replace_files() {
    let directory = tempfile::tempdir().unwrap();
    let base = directory.path().join("base.pdf");
    let revision = directory.path().join("revision.pdf");
    seed(&base, 0);
    seed(&revision, 0);
    let base_bytes = std::fs::read(&base).unwrap();
    let revision_bytes = std::fs::read(&revision).unwrap();
    let manifest = directory.path().join("manifest.json");
    let original = success(cli(&base, &revision, &manifest, "json").output().unwrap());
    let out = directory.path().join("refused.png");
    for args in [
        vec!["--base-page", "0"],
        vec!["--revision-page", "2"],
        vec!["--max-side", "63"],
        vec!["--max-side", "2049"],
        vec!["--scale", "0"],
        vec!["--scale", "101"],
        vec!["--scale", "NaN"],
        vec!["--rotation", "inf"],
        vec!["--rotation", "361"],
        vec!["--opacity", "-0.1"],
        vec!["--opacity", "1.1"],
        vec!["--translate", "1000001,0"],
        vec!["--translate", "0,NaN"],
        vec!["--base-points", "10,10 20,20"],
        vec!["--revision-points", "10,10 20,20"],
        vec![
            "--base-points",
            "10,10 10,10",
            "--revision-points",
            "10,10 20,20",
        ],
        vec![
            "--base-points",
            "10,10 9999,20",
            "--revision-points",
            "10,10 20,20",
        ],
        vec![
            "--base-points",
            "10,10 20,20",
            "--revision-points",
            "10,10 20,20",
            "--scale",
            "1",
        ],
        vec![
            "--base-points",
            "10,10 20,20",
            "--revision-points",
            "10,10 20,20",
            "--translate",
            "0,0",
        ],
        vec!["--expected-base-sha256", "short"],
    ] {
        failure(
            cli(&base, &revision, &out, "png")
                .args(args)
                .output()
                .unwrap(),
        );
        assert!(!out.exists());
    }
    for destination in [&base, &revision, &manifest] {
        let before = std::fs::read(destination).unwrap();
        assert_eq!(
            failure(
                cli(&base, &revision, destination, "png")
                    .arg("--overwrite")
                    .output()
                    .unwrap()
            )
            .unwrap()["error"],
            "already_exists"
        );
        assert_eq!(std::fs::read(destination).unwrap(), before);
    }
    // A geometry-preserving metadata rewrite still changes the exact source fingerprint.
    let mut changed = lopdf::Document::load(&revision).unwrap();
    let info =
        changed.add_object(lopdf::dictionary! {"Title"=>lopdf::text_string("Changed revision")});
    changed.trailer.set("Info", info);
    changed.save(&revision).unwrap();
    let expected = original["report"]["revision"]["source_sha256"]
        .as_str()
        .unwrap();
    assert_eq!(
        failure(
            cli(&base, &revision, &out, "png")
                .args(["--expected-revision-sha256", expected])
                .output()
                .unwrap()
        )
        .unwrap()["error"],
        "other"
    );
    assert!(!out.exists());
    let expected = original["report"]["base"]["source_sha256"]
        .as_str()
        .unwrap();
    assert_eq!(
        failure(
            cli(&revision, &base, &out, "png")
                .args(["--expected-base-sha256", expected])
                .output()
                .unwrap()
        )
        .unwrap()["error"],
        "other"
    );
    assert!(!out.exists());
    assert_eq!(std::fs::read(&base).unwrap(), base_bytes);
    assert_ne!(std::fs::read(&revision).unwrap(), revision_bytes);
    let oversized = directory.path().join("oversized.pdf");
    let huge = std::fs::File::create(&oversized).unwrap();
    huge.set_len(64 * 1024 * 1024 + 1).unwrap();
    drop(huge);
    failure(cli(&base, &oversized, &out, "png").output().unwrap());
    assert!(!out.exists());
}

fn protected(path: &Path, bytes: &[u8], opening: &str, owner: &str, aes256: bool, copy: bool) {
    let bytes = sunipdf_core::security::protect(
        bytes,
        None,
        &Protection {
            user_password: opening.into(),
            owner_password: owner.into(),
            allow_copy: copy,
            allow_print: false,
            allow_annotate: false,
            allow_modify: false,
            aes256,
        },
    )
    .unwrap();
    std::fs::write(path, bytes).unwrap();
}

#[test]
fn both_pdf_permissions_and_separate_password_env_stdin_sources_are_enforced() {
    let directory = tempfile::tempdir().unwrap();
    let plain = directory.path().join("plain.pdf");
    seed(&plain, 0);
    let bytes = std::fs::read(&plain).unwrap();
    let user_a = format!("overlay-base-{}", std::process::id());
    let owner_a = format!("overlay-base-owner-{}", std::process::id());
    let user_b = format!("overlay-revision-{}", std::process::id());
    let owner_b = format!("overlay-revision-owner-{}", std::process::id());
    let base = directory.path().join("base-aes128.pdf");
    let revision = directory.path().join("revision-aes256.pdf");
    protected(&base, &bytes, &user_a, &owner_a, false, true);
    protected(&revision, &bytes, &user_b, &owner_b, true, true);
    for mode in ["env", "base-stdin", "revision-stdin"] {
        let out = directory.path().join(format!("{mode}.json"));
        let mut command = cli(&base, &revision, &out, "json");
        let stdin = match mode {
            "base-stdin" => {
                command
                    .arg("--password-stdin")
                    .args(["--revision-password-env", "SUNIPDF_OVERLAY_REVISION"])
                    .env("SUNIPDF_OVERLAY_REVISION", &user_b);
                Some(user_a.as_bytes())
            }
            "revision-stdin" => {
                command
                    .args(["--password-env", "SUNIPDF_OVERLAY_BASE"])
                    .env("SUNIPDF_OVERLAY_BASE", &user_a)
                    .arg("--revision-password-stdin");
                Some(user_b.as_bytes())
            }
            _ => {
                command
                    .args([
                        "--password-env",
                        "SUNIPDF_OVERLAY_BASE",
                        "--revision-password-env",
                        "SUNIPDF_OVERLAY_REVISION",
                    ])
                    .env("SUNIPDF_OVERLAY_BASE", &user_a)
                    .env("SUNIPDF_OVERLAY_REVISION", &user_b);
                None
            }
        };
        assert!(!command
            .get_args()
            .any(|arg| arg == user_a.as_str() || arg == user_b.as_str()));
        let output = if let Some(bytes) = stdin {
            piped(command, bytes)
        } else {
            command.output().unwrap()
        };
        let text = String::from_utf8_lossy(&output.stdout).to_string();
        success(output);
        assert!(!text.contains(&user_a) && !text.contains(&user_b));
    }
    let refused = directory.path().join("refused.png");
    let mut duplicate = cli(&base, &revision, &refused, "png");
    duplicate.args(["--password-stdin", "--revision-password-stdin"]);
    failure(piped(duplicate, b"unused"));
    assert!(!refused.exists());
    let mut wrong = cli(&base, &revision, &refused, "png");
    wrong
        .args([
            "--password-env",
            "SUNIPDF_OVERLAY_BASE",
            "--revision-password-env",
            "SUNIPDF_OVERLAY_REVISION",
        ])
        .env("SUNIPDF_OVERLAY_BASE", &user_a)
        .env("SUNIPDF_OVERLAY_REVISION", "invalid-synthetic-value");
    assert_eq!(
        failure(wrong.output().unwrap()).unwrap()["error"],
        "password_required"
    );
    assert!(!refused.exists());
    for (side, copy_a, copy_b) in [("base", false, true), ("revision", true, false)] {
        protected(&base, &bytes, &user_a, &owner_a, false, copy_a);
        protected(&revision, &bytes, &user_b, &owner_b, true, copy_b);
        let base_before = std::fs::read(&base).unwrap();
        let revision_before = std::fs::read(&revision).unwrap();
        for (label, a, b) in [("user", &user_a, &user_b), ("owner", &owner_a, &owner_b)] {
            let out = directory.path().join(format!("refused-{side}-{label}.png"));
            let mut command = cli(&base, &revision, &out, "png");
            command
                .args([
                    "--password-env",
                    "SUNIPDF_OVERLAY_BASE",
                    "--revision-password-env",
                    "SUNIPDF_OVERLAY_REVISION",
                ])
                .env("SUNIPDF_OVERLAY_BASE", a)
                .env("SUNIPDF_OVERLAY_REVISION", b);
            assert_eq!(
                failure(command.output().unwrap()).unwrap()["error"],
                "security_restricted"
            );
            assert!(!out.exists());
        }
        assert_eq!(std::fs::read(&base).unwrap(), base_before);
        assert_eq!(std::fs::read(&revision).unwrap(), revision_before);
    }
}
