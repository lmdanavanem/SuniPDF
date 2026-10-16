//! `add-image` de bout en bout (binaire lancé, fichier rouvert par `render`) : exige `pdfium.dll`
//! (scripts/fetch-pdfium.ps1), comme les tests d'intégration du moteur.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"))
}

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf")
}

fn stderr_json(o: &Output) -> serde_json::Value {
    serde_json::from_slice(&o.stderr)
        .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&o.stderr)))
}

fn stdout_json(o: &Output) -> serde_json::Value {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    serde_json::from_slice(&o.stdout).unwrap()
}

fn batch_image(source: &Path, image: &Path, out: &Path, pages: &str) -> Command {
    let mut command = cli();
    command
        .arg("add-image")
        .arg(source)
        .arg("--image")
        .arg(image)
        .args(["--pages", pages, "--x", "100", "--y", "100", "--out"])
        .arg(out);
    command
}

#[test]
fn add_image_batch_round_trips_mixed_and_encrypted_pages() {
    let engine = sunipdf_core::Engine::new().unwrap();
    batch_fits_mixed_pages_once_and_keeps_unselected_pages(&engine);
    batch_owner_open_retains_aes_protection_and_readable_content(&engine);
}

fn batch_fits_mixed_pages_once_and_keeps_unselected_pages(engine: &sunipdf_core::Engine) {
    use sunipdf_core::ObjectKind;
    let temp = tempfile::tempdir().unwrap();
    let source = fixture().with_file_name("boxes.pdf");
    let original = std::fs::read(&source).unwrap();
    let image = temp.path().join("signature.png");
    image::RgbaImage::from_pixel(80, 40, image::Rgba([30, 180, 50, 255]))
        .save(&image)
        .unwrap();
    let out = temp.path().join("selected.pdf");
    let mut command = batch_image(&source, &image, &out, "2-3,3,2");
    command.args(["--width", "2000"]);
    let report = stdout_json(&command.output().unwrap());
    assert_eq!(report["pages"], serde_json::json!([2, 3]));
    assert_eq!(report["count"], 2);
    assert_eq!(report["digital_signature"], false);
    assert!(report.get("page").is_none());
    let before = engine.open(&source, None).unwrap();
    let after = engine.open(&out, None).unwrap();
    assert_eq!(after.page_count(), before.page_count());
    for page in 0..before.page_count() {
        let selected = (1..=2).contains(&page);
        let objects = after.page_objects(page).unwrap();
        assert_eq!(
            objects.len(),
            before.page_objects(page).unwrap().len() + usize::from(selected)
        );
        let rendered_before = before.render_page(page, 0.5).unwrap();
        let rendered_after = after.render_page(page, 0.5).unwrap();
        assert_eq!(rendered_before.rgba == rendered_after.rgba, !selected);
        if selected {
            let mark = objects.last().unwrap();
            assert_eq!(mark.kind, ObjectKind::Image);
            let b = mark.bounds;
            let size = after.page_size(page).unwrap();
            assert!(b.left >= -0.02 && b.bottom >= -0.02, "{b:?}");
            assert!(b.right <= size.width_pt + 0.02 && b.top <= size.height_pt + 0.02);
            assert!(((b.right - b.left) / (b.top - b.bottom) - 2.0).abs() < 0.01);
        }
    }
    let all = temp.path().join("all.pdf");
    let report = stdout_json(&batch_image(&source, &image, &all, "all").output().unwrap());
    assert_eq!(report["pages"], serde_json::json!([1, 2, 3, 4]));
    assert_eq!(report["count"], 4);
    let all = engine.open(&all, None).unwrap();
    for page in 0..before.page_count() {
        assert_eq!(
            all.page_objects(page).unwrap().len(),
            before.page_objects(page).unwrap().len() + 1
        );
    }
    assert_eq!(std::fs::read(&source).unwrap(), original);
}

fn batch_owner_open_retains_aes_protection_and_readable_content(engine: &sunipdf_core::Engine) {
    use sunipdf_core::{security, Protection};
    let temp = tempfile::tempdir().unwrap();
    let image = temp.path().join("initials.png");
    image::RgbaImage::from_pixel(40, 20, image::Rgba([20, 20, 20, 255]))
        .save(&image)
        .unwrap();
    let input = std::fs::read(fixture()).unwrap();
    for aes256 in [false, true] {
        let original = security::protect(
            &input,
            None,
            &Protection {
                user_password: "package-user".into(),
                owner_password: "package-owner".into(),
                allow_copy: false,
                allow_print: false,
                aes256,
                ..Default::default()
            },
        )
        .unwrap();
        let source = temp.path().join(format!("protected-{aes256}.pdf"));
        std::fs::write(&source, &original).unwrap();
        let out = temp.path().join(format!("marked-{aes256}.pdf"));
        let mut command = batch_image(&source, &image, &out, "all");
        command.args(["--password", "package-owner"]);
        assert_eq!(stdout_json(&command.output().unwrap())["count"], 2);
        let mut properties = cli();
        properties
            .arg("properties")
            .arg(&out)
            .args(["--password", "package-user"]);
        let properties = stdout_json(&properties.output().unwrap());
        assert_eq!(properties["encrypted"], true);
        assert_eq!(properties["permissions"]["copy"], false);
        assert_eq!(properties["permissions"]["print"], false);
        assert!(engine.open(&out, None).is_err());
        let before = engine.open(&source, Some("package-user")).unwrap();
        for password in ["package-user", "package-owner"] {
            let after = engine.open(&out, Some(password)).unwrap();
            for page in 0..2 {
                assert_eq!(
                    after.page_objects(page).unwrap().len(),
                    before.page_objects(page).unwrap().len() + 1
                );
                assert_eq!(
                    after.page_text(page).unwrap(),
                    before.page_text(page).unwrap()
                );
                assert_ne!(
                    after.render_page(page, 0.5).unwrap().rgba,
                    before.render_page(page, 0.5).unwrap().rgba
                );
            }
        }
        assert_eq!(std::fs::read(&source).unwrap(), original);
    }
}

#[test]
fn add_image_batch_invalid_input_never_writes_or_overwrites() {
    let temp = tempfile::tempdir().unwrap();
    let source = fixture();
    let original = std::fs::read(&source).unwrap();
    let image = temp.path().join("initials.png");
    image::RgbaImage::from_pixel(40, 20, image::Rgba([20, 20, 20, 255]))
        .save(&image)
        .unwrap();
    for (index, (pages, x, y, width, explicit_page)) in [
        ("0", "100", "100", "150", false),
        ("3", "100", "100", "150", false),
        ("2-1", "100", "100", "150", false),
        ("invalid", "100", "100", "150", false),
        ("", "100", "100", "150", false),
        ("all", "100", "100", "150", true),
        ("all", "100", "100", "0", false),
        ("all", "100", "100", "NaN", false),
        ("all", "100", "100", "inf", false),
        ("all", "NaN", "100", "150", false),
        ("all", "100", "inf", "150", false),
    ]
    .into_iter()
    .enumerate()
    {
        let out = temp.path().join(format!("invalid-{index}.pdf"));
        let mut command = cli();
        command
            .arg("add-image")
            .arg(&source)
            .arg("--image")
            .arg(&image)
            .args(["--pages", pages, "--x", x, "--y", y, "--width", width])
            .arg("--out")
            .arg(&out);
        if explicit_page {
            command.args(["--page", "1"]);
        }
        let output = command.output().unwrap();
        assert!(!output.status.success(), "invalid case {index}");
        assert!(!out.exists(), "invalid case {index} created output");
        std::fs::write(&out, b"keep existing output").unwrap();
        command.arg("--overwrite");
        assert!(!command.output().unwrap().status.success());
        assert_eq!(std::fs::read(&out).unwrap(), b"keep existing output");
    }
    assert_eq!(std::fs::read(&source).unwrap(), original);
}

#[test]
fn add_image_removes_background_trims_and_renders() {
    let dir = std::env::temp_dir().join(format!("sunipdf-cli-add-image-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // « Photo » 200 × 100 sur papier presque blanc : trait sombre x 50..150, y 40..60.
    let photo = dir.join("photo.png");
    image::RgbaImage::from_fn(200, 100, |x, y| {
        if (50..150).contains(&x) && (40..60).contains(&y) {
            image::Rgba([20, 20, 20, 255])
        } else {
            image::Rgba([250, 250, 250, 255])
        }
    })
    .save(&photo)
    .unwrap();
    let out = dir.join("out.pdf");
    let add = || {
        cli()
            .arg("add-image")
            .arg(fixture())
            .arg("--image")
            .arg(&photo)
            .args([
                "--x",
                "300",
                "--y",
                "100",
                "--width",
                "100",
                "--remove-background",
                "200",
                "--out",
            ])
            .arg(&out)
            .output()
            .unwrap()
    };
    let o = add();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["page"], 1);
    assert!(v.get("pages").is_none());
    assert!(v.get("count").is_none());
    assert_eq!(v["background_removed"], true);
    assert_eq!(v["digital_signature"], false);
    // Rognée au trait plus 4 px de marge : 108 × 28 px pour 100 pt de large.
    let h = v["height_pt"].as_f64().unwrap();
    assert!((h - 100.0 * 28.0 / 108.0).abs() < 0.01, "height_pt {h}");

    // Le fichier produit se rouvre ; le trait est sombre, la marge (fond retiré) laisse la page
    // blanche. Origine PDF en bas : écran y = 792 − y.
    let png = dir.join("page.png");
    let o = cli()
        .arg("render")
        .arg(&out)
        .args(["--page", "1", "--scale", "1", "--out"])
        .arg(&png)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let page = image::open(&png).unwrap().into_rgba8();
    let ink = page.get_pixel(350, 792 - 113).0;
    assert!(ink[0] < 80 && ink[1] < 80 && ink[2] < 80, "trait {ink:?}");
    let margin = page.get_pixel(301, 792 - 102).0;
    assert_eq!(&margin[..3], &[255, 255, 255], "marge {margin:?}");

    // Sortie existante : refusée sans --overwrite (code 6).
    let o = add();
    assert_eq!(o.status.code(), Some(6));
    assert_eq!(stderr_json(&o)["error"], "already_exists");

    // Image sans encre une fois le fond retiré : erreur explicite, rien d'écrit.
    let blank = dir.join("blank.png");
    image::RgbaImage::from_pixel(50, 50, image::Rgba([255, 255, 255, 255]))
        .save(&blank)
        .unwrap();
    let out2 = dir.join("out2.pdf");
    let o = cli()
        .arg("add-image")
        .arg(fixture())
        .arg("--image")
        .arg(&blank)
        .args([
            "--x",
            "10",
            "--y",
            "10",
            "--remove-background",
            "200",
            "--out",
        ])
        .arg(&out2)
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    let e = stderr_json(&o);
    assert_eq!(e["error"], "other");
    assert!(
        e["message"]
            .as_str()
            .unwrap()
            .contains("empty after background removal"),
        "{e}"
    );
    assert!(!out2.exists());

    // PNG entièrement transparent sans --remove-background : objet invisible refusé.
    let clear = dir.join("clear.png");
    image::RgbaImage::new(50, 50).save(&clear).unwrap();
    let out3 = dir.join("out3.pdf");
    let o = cli()
        .arg("add-image")
        .arg(fixture())
        .arg("--image")
        .arg(&clear)
        .args(["--x", "10", "--y", "10", "--out"])
        .arg(&out3)
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    let e = stderr_json(&o);
    assert_eq!(e["error"], "other");
    assert!(
        e["message"].as_str().unwrap().contains("fully transparent"),
        "{e}"
    );
    assert!(!out3.exists());
    let _ = std::fs::remove_dir_all(&dir);
}
