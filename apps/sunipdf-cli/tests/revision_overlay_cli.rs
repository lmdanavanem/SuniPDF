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

fn stack(manifest: &Path, out: &Path, format: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
    command
        .arg("revision-overlay-stack")
        .arg("--manifest")
        .arg(manifest)
        .args(["--format", format, "--out"])
        .arg(out);
    command
}

#[test]
fn saved_overlay_setups_relocate_with_sources_preserve_pixels_and_refuse_changed_hidden_layers() {
    use sunipdf_core::revision_overlay::recipe;
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let names = ["base.pdf", "revision.pdf", "hidden.pdf", "detail.pdf"];
    let mut originals = Vec::new();
    for (index, name) in names.iter().enumerate() {
        let path = root.join(name);
        seed(&path, index as i64 * 90);
        originals.push(std::fs::read(path).unwrap());
    }
    let input = root.join("input.json");
    let value = serde_json::json!({"version":1,"long_side":128,"layers":[
        {"path":names[0],"color":[180,30,40],"opacity":0.9},
        {"path":names[1],"color":[20,130,230],"opacity":0.4,
         "alignment":{"mode":"manual","transform":{"scale":0.8,"rotation_degrees":13.0,"translation":[15.0,-8.0]}}},
        {"path":names[2],"visible":false},
        {"path":names[3],"color":[30,190,80],"opacity":0.6}
    ]});
    std::fs::write(&input, serde_json::to_vec(&value).unwrap()).unwrap();
    let expected = root.join("expected.png");
    let before = success(stack(&input, &expected, "png").output().unwrap());
    let setup = root.join("review.setup.json");
    let saved = success(stack(&input, &setup, "setup").output().unwrap());
    assert_eq!(saved["format"], "setup");
    assert_eq!(saved["originals_unchanged"], true);
    let setup_bytes = std::fs::read(&setup).unwrap();
    let loaded = recipe::parse(&setup_bytes).unwrap();
    recipe::require_desktop(&loaded).unwrap();
    assert_eq!(loaded.layers.len(), 4);
    assert_eq!(loaded.long_side, 128);
    for (index, layer) in loaded.layers.iter().enumerate() {
        assert_eq!(layer.path, Path::new(names[index]));
        assert_eq!(
            layer.expected_sha256.as_deref(),
            before["report"]["layers"][index]["source"]["source_sha256"].as_str()
        );
        assert!(layer.password_env.is_none());
    }
    assert_eq!(loaded.layers[2].visible, Some(false));
    assert!(!String::from_utf8_lossy(&setup_bytes).contains("password_env"));
    let moved = root.join("relocated");
    std::fs::create_dir(&moved).unwrap();
    for (name, original) in names.iter().zip(&originals) {
        std::fs::write(moved.join(name), original).unwrap();
    }
    let moved_setup = moved.join("review.setup.json");
    std::fs::write(&moved_setup, &setup_bytes).unwrap();
    let moved_png = moved.join("same.png");
    success(stack(&moved_setup, &moved_png, "png").output().unwrap());
    assert_eq!(png(&moved_png), png(&expected));
    failure(
        stack(&input, &setup, "setup")
            .arg("--overwrite")
            .output()
            .unwrap(),
    );
    assert_eq!(std::fs::read(&setup).unwrap(), setup_bytes);
    let mut changed = originals[2].clone();
    changed.extend_from_slice(b"\n% Changed hidden source\n");
    std::fs::write(moved.join(names[2]), changed).unwrap();
    for format in ["png", "json", "setup"] {
        let refused = moved.join(format!("refused-{format}"));
        failure(stack(&moved_setup, &refused, format).output().unwrap());
        assert!(!refused.exists());
    }
    for (name, original) in names.iter().zip(originals) {
        assert_eq!(std::fs::read(root.join(name)).unwrap(), original);
    }
}

#[test]
fn saved_overlay_setup_omits_protected_layer_credentials_and_requires_them_again() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let source = root.join("plain.pdf");
    seed(&source, 0);
    let plain = std::fs::read(&source).unwrap();
    let user = format!("overlay-setup-user-{}", std::process::id());
    let owner = format!("overlay-setup-owner-{}", std::process::id());
    let encrypted = root.join("protected.pdf");
    protected(&encrypted, &plain, &user, &owner, true, true);
    let input = root.join("protected-input.json");
    std::fs::write(
        &input,
        serde_json::to_vec(&serde_json::json!({"version":1,"long_side":128,"layers":[
            {"path":"plain.pdf"},
            {"path":"protected.pdf","password_env":"SUNIPDF_SETUP_TEST_SECRET"}
        ]}))
        .unwrap(),
    )
    .unwrap();
    let setup = root.join("protected.setup.json");
    success(
        stack(&input, &setup, "setup")
            .env("SUNIPDF_SETUP_TEST_SECRET", &user)
            .output()
            .unwrap(),
    );
    let bytes = std::fs::read(&setup).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    for secret in [
        user.as_str(),
        owner.as_str(),
        "SUNIPDF_SETUP_TEST_SECRET",
        "password_env",
    ] {
        assert!(!text.contains(secret));
    }
    let manifest = sunipdf_core::revision_overlay::recipe::parse(&bytes).unwrap();
    sunipdf_core::revision_overlay::recipe::require_desktop(&manifest).unwrap();
    let output = root.join("needs-password.png");
    failure(
        stack(&setup, &output, "png")
            .env("SUNIPDF_SETUP_TEST_SECRET", &user)
            .output()
            .unwrap(),
    );
    assert!(!output.exists());
    assert_eq!(std::fs::read(&setup).unwrap(), bytes);
    assert_eq!(std::fs::read(&source).unwrap(), plain);
}

#[test]
fn stack_manifest_preserves_legacy_pixels_and_controls_each_added_layer() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let source = root.join("page.pdf");
    seed(&source, 90);
    let original = std::fs::read(&source).unwrap();
    let legacy = root.join("legacy.png");
    success(cli(&source, &source, &legacy, "png").output().unwrap());
    let manifest = root.join("layers.json");
    let mut recipe = serde_json::json!({"version":1,"long_side":128,"layers":[{"path":"page.pdf"},{"path":"page.pdf"}]});
    std::fs::write(&manifest, serde_json::to_vec(&recipe).unwrap()).unwrap();
    let two = root.join("two.png");
    let result = success(stack(&manifest, &two, "png").output().unwrap());
    assert_eq!(png(&two), png(&legacy));
    assert_eq!(result["report"]["schema"], "sunipdf-revision-overlay-stack");
    assert_eq!(result["report"]["layers"].as_array().unwrap().len(), 2);
    let hash = result["report"]["layers"][0]["source"]["source_sha256"].clone();
    recipe["layers"][0]["expected_sha256"] = hash;
    recipe["layers"].as_array_mut().unwrap().extend([
        serde_json::json!({"path":"page.pdf","color":[0,180,70],"opacity":0.7}),
        serde_json::json!({"path":"page.pdf","color":[255,140,0],"opacity":0.5}),
    ]);
    std::fs::write(&manifest, serde_json::to_vec(&recipe).unwrap()).unwrap();
    let four = root.join("four.png");
    let result = success(stack(&manifest, &four, "png").output().unwrap());
    assert_eq!(result["report"]["layers"].as_array().unwrap().len(), 4);
    assert_ne!(png(&four), png(&two));
    let report = root.join("four.json");
    let exported = success(stack(&manifest, &report, "json").output().unwrap());
    let saved: Value = serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
    assert_eq!(saved, exported["report"]);
    assert_eq!(saved, result["report"]);
    recipe["layers"][2]["visible"] = false.into();
    recipe["layers"][3]["opacity"] = 0.into();
    std::fs::write(&manifest, serde_json::to_vec(&recipe).unwrap()).unwrap();
    let isolated = root.join("isolated.png");
    success(stack(&manifest, &isolated, "png").output().unwrap());
    assert_eq!(png(&isolated), png(&two));
    assert_eq!(std::fs::read(&source).unwrap(), original);
    let before = std::fs::read(&four).unwrap();
    failure(
        stack(&manifest, &four, "png")
            .arg("--overwrite")
            .output()
            .unwrap(),
    );
    assert_eq!(std::fs::read(&four).unwrap(), before);
    recipe["layers"][0]["expected_sha256"] = "0".repeat(64).into();
    std::fs::write(&manifest, serde_json::to_vec(&recipe).unwrap()).unwrap();
    let refused = root.join("stale.png");
    failure(stack(&manifest, &refused, "png").output().unwrap());
    assert!(!refused.exists());
}

#[test]
fn stack_manifest_checks_hidden_layer_permissions_and_independent_secret_sources() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let source = root.join("page.pdf");
    seed(&source, 0);
    let bytes = std::fs::read(&source).unwrap();
    let user = format!("stack-user-{}", std::process::id());
    let owner = format!("stack-owner-{}", std::process::id());
    let protected_path = root.join("protected.pdf");
    protected(&protected_path, &bytes, &user, &owner, true, true);
    let restricted_path = root.join("restricted.pdf");
    protected(&restricted_path, &bytes, &user, &owner, false, false);
    let manifest = root.join("layers.json");
    let mut recipe = serde_json::json!({"version":1,"long_side":128,"layers":[
        {"path":"page.pdf"},
        {"path":"protected.pdf","password_env":"SUNIPDF_STACK_TEST_SECRET"}
    ]});
    std::fs::write(&manifest, serde_json::to_vec(&recipe).unwrap()).unwrap();
    let output = root.join("allowed.png");
    let result = success(
        stack(&manifest, &output, "png")
            .env("SUNIPDF_STACK_TEST_SECRET", &user)
            .output()
            .unwrap(),
    );
    assert!(!result.to_string().contains(&user));
    assert!(!result.to_string().contains("SUNIPDF_STACK_TEST_SECRET"));
    recipe["layers"][1]["path"] = "restricted.pdf".into();
    recipe["layers"][1]["visible"] = false.into();
    std::fs::write(&manifest, serde_json::to_vec(&recipe).unwrap()).unwrap();
    let refused = root.join("refused.png");
    failure(
        stack(&manifest, &refused, "png")
            .env("SUNIPDF_STACK_TEST_SECRET", &user)
            .output()
            .unwrap(),
    );
    assert!(!refused.exists());
    failure(
        stack(&manifest, &refused, "png")
            .env_remove("SUNIPDF_STACK_TEST_SECRET")
            .output()
            .unwrap(),
    );
    assert!(!refused.exists());
    failure(
        stack(&manifest, &refused, "png")
            .args(["--password-env", "SUNIPDF_STACK_TEST_SECRET"])
            .env("SUNIPDF_STACK_TEST_SECRET", &user)
            .output()
            .unwrap(),
    );
    assert!(!refused.exists());
    assert_eq!(std::fs::read(source).unwrap(), bytes);
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
