use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use sunipdf_core::Engine;

#[path = "../../../crates/sunipdf-core/tests/support/conversion_images.rs"]
mod samples;

fn cli(images: &[PathBuf], out: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
    command.arg("images2pdf").args(images).arg("--out").arg(out);
    command
}

fn success(output: Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn conversion_cli_preserves_all_jpegs_orientation_transparency_sources_and_output_guards() {
    // All native readback cases share one engine instance in this process.
    let engine = Engine::new().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let mut inputs = Vec::new();
    for exif in 1..=8 {
        let path = temp.path().join(format!("exif-{exif}.jpg"));
        samples::jpeg(&path, exif);
        inputs.push(path);
    }
    let png = temp.path().join("transparent.png");
    samples::transparent_png(&png);
    inputs.push(png);
    let originals: Vec<_> = inputs
        .iter()
        .map(|path| std::fs::read(path).unwrap())
        .collect();
    let out = temp.path().join("converted.pdf");
    let report = success(cli(&inputs, &out).args(["--dpi", "72"]).output().unwrap());
    assert_eq!(report["page_count"], 9);
    assert_eq!(report["out"], out.to_string_lossy().as_ref());
    let pdf = lopdf::Document::load(&out).unwrap();
    let document = engine.open(&out, None).unwrap();
    for (page, original) in originals[..8].iter().enumerate() {
        samples::assert_jpeg_bytes(&pdf, page, original);
        let exif = page as u8 + 1;
        let mut expected = image::open(&inputs[page]).unwrap();
        expected.apply_orientation(image::metadata::Orientation::from_exif(exif).unwrap());
        let rendered = document.render_page(page, 1.0).unwrap();
        assert_eq!(
            (rendered.width, rendered.height),
            (expected.width(), expected.height())
        );
        let expected = expected.to_rgb8();
        for (x, y) in [
            (8, 8),
            (rendered.width - 9, 8),
            (8, rendered.height - 9),
            (rendered.width - 9, rendered.height - 9),
        ] {
            let offset = ((y * rendered.width + x) * 4) as usize;
            for (actual, expected) in rendered.rgba[offset..offset + 3]
                .iter()
                .zip(expected.get_pixel(x, y).0)
            {
                assert!(
                    actual.abs_diff(expected) < 25,
                    "EXIF {exif} corner ({x},{y})"
                );
            }
        }
    }
    let rendered = document.render_page(8, 1.0).unwrap();
    let clear = (16 * 48 + 40) * 4;
    assert!(rendered.rgba[clear..clear + 3]
        .iter()
        .all(|channel| *channel > 245));
    let before = std::fs::read(&out).unwrap();
    assert_eq!(cli(&inputs, &out).output().unwrap().status.code(), Some(6));
    assert_eq!(std::fs::read(&out).unwrap(), before);
    let default = temp.path().join("default-dpi.pdf");
    assert_eq!(
        success(cli(&inputs[..1], &default).output().unwrap())["page_count"],
        1
    );
    let size = engine.open(&default, None).unwrap().page_size(0).unwrap();
    assert!((size.width_pt - 48.0 * 72.0 / 150.0).abs() < 0.001);
    for (path, original) in inputs.iter().zip(originals) {
        assert_eq!(std::fs::read(path).unwrap(), original);
    }
}

#[test]
fn conversion_cli_budget_and_geometry_fail_atomically_even_with_overwrite() {
    let temp = tempfile::tempdir().unwrap();
    let header = temp.path().join("large-header.png");
    let bytes = samples::png_header(4096, 4096, 8, 6);
    std::fs::write(&header, &bytes).unwrap();
    let inputs = vec![header.clone(); 9];
    for overwrite in [false, true] {
        let out = temp.path().join(format!("budget-{overwrite}.pdf"));
        let sentinel = b"existing output must survive";
        if overwrite {
            std::fs::write(&out, sentinel).unwrap();
        }
        let mut command = cli(&inputs, &out);
        if overwrite {
            command.arg("--overwrite");
        }
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"], "image_batch_too_large");
        assert!(output.stdout.is_empty());
        if overwrite {
            assert_eq!(std::fs::read(&out).unwrap(), sentinel);
        } else {
            assert!(!out.exists());
        }
    }
    assert_eq!(std::fs::read(&header).unwrap(), bytes);
    let jpeg = temp.path().join("source.jpg");
    samples::jpeg(&jpeg, 5);
    let original = std::fs::read(&jpeg).unwrap();
    let out = temp.path().join("geometry.pdf");
    for dpi in ["0", "NaN", "inf", "1e-45"] {
        let sentinel = b"valid existing output";
        std::fs::write(&out, sentinel).unwrap();
        let output = cli(std::slice::from_ref(&jpeg), &out)
            .args(["--overwrite", "--dpi", dpi])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"], "other");
        assert_eq!(std::fs::read(&out).unwrap(), sentinel);
    }
    assert_eq!(std::fs::read(&jpeg).unwrap(), original);
    let no_output = temp.path().join("truncated.pdf");
    let output = cli(&[header], &no_output).output().unwrap();
    assert!(!output.status.success());
    assert!(!no_output.exists());
}
