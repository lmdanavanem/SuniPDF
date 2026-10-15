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
    let _ = std::fs::remove_dir_all(&dir);
}
