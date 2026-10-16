//! Real CLI save/reopen coverage for compression, choice fields, interchange and grouped transforms.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"))
}
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/sunipdf-core/tests/fixtures")
        .join(name)
}
fn json(output: Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn failed(output: Output, out: &Path) {
    assert!(!output.status.success(), "unexpected success");
    assert!(!out.exists(), "failure created an output");
    assert!(
        serde_json::from_slice::<serde_json::Value>(&output.stderr).unwrap()["error"].is_string()
    );
}
fn values(path: &Path) -> serde_json::Value {
    json(
        cli()
            .arg("form-fields")
            .arg(path)
            .arg("--values-only")
            .output()
            .unwrap(),
    )
}
fn step(input: &Path, out: &Path, command: &str, args: &[&str]) -> serde_json::Value {
    json(
        cli()
            .arg(command)
            .arg(input)
            .args(args)
            .arg("--out")
            .arg(out)
            .output()
            .unwrap(),
    )
}

#[test]
fn editable_combo_creation_accepts_custom_text_and_rejects_invalid_flag_without_output() {
    let temp = tempfile::tempdir().unwrap();
    let input = fixture("boxes.pdf");
    let original = std::fs::read(&input).unwrap();
    let args = [
        "--kind",
        "combo",
        "--name",
        "Country",
        "--rect",
        "40,40,220,65",
        "--page",
        "3",
        "--option",
        "FR=France",
        "--option",
        "BE=Belgique",
    ];
    let fixed = temp.path().join("fixed.pdf");
    step(&input, &fixed, "add-form-field", &args);
    let rejected = temp.path().join("rejected.pdf");
    failed(
        cli()
            .arg("fill-form")
            .arg(&fixed)
            .args(["--set", "Country=Suisse romande", "--out"])
            .arg(&rejected)
            .output()
            .unwrap(),
        &rejected,
    );
    for kind in ["text", "checkbox", "radio", "list"] {
        let invalid = temp.path().join(format!("invalid-{kind}.pdf"));
        let mut command = cli();
        command.arg("add-form-field").arg(&input).args([
            "--kind",
            kind,
            "--name",
            "Invalid",
            "--rect",
            "40,40,220,100",
            "--editable",
        ]);
        if matches!(kind, "radio" | "list") {
            command.args(["--option", "one=First", "--option", "two=Second"]);
        }
        failed(
            command.arg("--out").arg(&invalid).output().unwrap(),
            &invalid,
        );
    }
    let created = temp.path().join("editable.pdf");
    let mut args = args.to_vec();
    args.push("--editable");
    step(&input, &created, "add-form-field", &args);
    let info = json(cli().arg("form-fields").arg(&created).output().unwrap());
    assert_eq!(info["fields"][0]["editable"], true);
    assert_eq!(info["fields"][0]["widgets"][0]["page"], 3);
    let filled = temp.path().join("filled.pdf");
    step(
        &created,
        &filled,
        "fill-form",
        &["--set", "Country=Suisse romande – été"],
    );
    assert_eq!(values(&filled)["Country"], "Suisse romande – été");
    for format in ["json", "xfdf"] {
        let export = temp.path().join(format!("values.{format}"));
        step(&filled, &export, "form-export", &[]);
        let imported = temp.path().join(format!("imported-{format}.pdf"));
        json(
            cli()
                .arg("fill-form")
                .arg(&created)
                .arg("--values")
                .arg(&export)
                .arg("--out")
                .arg(&imported)
                .output()
                .unwrap(),
        );
        assert_eq!(values(&imported), values(&filled));
    }
    let listed = temp.path().join("listed.pdf");
    step(
        &filled,
        &listed,
        "fill-form",
        &["--set", "Country=Belgique"],
    );
    assert_eq!(values(&listed)["Country"], "BE");
    assert_eq!(std::fs::read(input).unwrap(), original);
}

#[test]
fn compression_reduces_images_and_preserves_text_links_fields_security_and_original() {
    let temp = tempfile::tempdir().unwrap();
    let image = temp.path().join("scan.jpg");
    image::RgbImage::from_fn(1400, 1000, |x, y| {
        image::Rgb([
            ((x * 37 + y * 23) % 256) as u8,
            ((x * 17 + y * 53) % 256) as u8,
            ((x * 11 + y * 41) % 256) as u8,
        ])
    })
    .save(&image)
    .unwrap();
    let scan = temp.path().join("scan.pdf");
    json(
        cli()
            .arg("images2pdf")
            .arg(&image)
            .arg("--out")
            .arg(&scan)
            .output()
            .unwrap(),
    );
    let text = temp.path().join("text.pdf");
    step(
        &scan,
        &text,
        "add-text",
        &[
            "--x",
            "30",
            "--y",
            "300",
            "--text",
            "Preserved compression text",
        ],
    );
    let link = temp.path().join("link.pdf");
    step(
        &text,
        &link,
        "annotate",
        &[
            "--kind",
            "link",
            "--rect",
            "30,260,190,285",
            "--url",
            "https://example.com/",
        ],
    );
    let source = temp.path().join("source.pdf");
    step(
        &link,
        &source,
        "add-form-field",
        &["--kind", "text", "--name", "Name", "--rect", "30,30,190,60"],
    );
    let original = std::fs::read(&source).unwrap();
    let out = temp.path().join("compressed.pdf");
    let report = step(
        &source,
        &out,
        "compress-images",
        &["--max-image-side", "256", "--jpeg-quality", "45"],
    );
    assert_eq!(report["images_changed"], 1);
    assert!(report["after"].as_u64().unwrap() < report["before"].as_u64().unwrap());
    assert_eq!(values(&source), values(&out));
    let before = cli().arg("text").arg(&source).output().unwrap();
    let after = cli().arg("text").arg(&out).output().unwrap();
    assert!(before.status.success() && after.status.success());
    assert_eq!(before.stdout, after.stdout);
    let annotations = |path: &Path| json(cli().arg("annotations").arg(path).output().unwrap());
    assert_eq!(annotations(&source), annotations(&out));
    let render = temp.path().join("compressed.png");
    step(&out, &render, "render", &["--page", "1", "--scale", "0.5"]);
    assert!(image::open(&render).unwrap().width() > 100);
    let attempted = cli()
        .arg("compress-images")
        .arg(&source)
        .arg("--out")
        .arg(&source)
        .arg("--overwrite")
        .output()
        .unwrap();
    assert!(!attempted.status.success());
    assert_eq!(std::fs::read(&source).unwrap(), original);
    let invalid = temp.path().join("invalid.pdf");
    failed(
        cli()
            .arg("compress-images")
            .arg(&source)
            .args(["--max-image-side", "0", "--out"])
            .arg(&invalid)
            .output()
            .unwrap(),
        &invalid,
    );

    let encrypted = temp.path().join("encrypted.pdf");
    step(
        &source,
        &encrypted,
        "protect",
        &[
            "--user-password",
            "reader",
            "--owner-password",
            "owner-secret",
        ],
    );
    let protected_out = temp.path().join("protected-compressed.pdf");
    step(
        &encrypted,
        &protected_out,
        "compress-images",
        &["--password", "reader", "--max-image-side", "256"],
    );
    assert!(!cli()
        .arg("info")
        .arg(&protected_out)
        .output()
        .unwrap()
        .status
        .success());
    let properties = json(
        cli()
            .arg("properties")
            .arg(&protected_out)
            .args(["--password", "reader"])
            .output()
            .unwrap(),
    );
    assert_eq!(properties["encrypted"], true);
    let denied = temp.path().join("wrong-password.pdf");
    failed(
        cli()
            .arg("compress-images")
            .arg(&encrypted)
            .args(["--password", "wrong", "--out"])
            .arg(&denied)
            .output()
            .unwrap(),
        &denied,
    );
}

#[test]
fn radio_combo_and_multiselect_list_round_trip_xfdf_fdf_and_grouped_values() {
    let temp = tempfile::tempdir().unwrap();
    let base = fixture("boxes.pdf");
    let original = std::fs::read(&base).unwrap();
    let radio = temp.path().join("radio.pdf");
    step(
        &base,
        &radio,
        "add-form-field",
        &[
            "--kind",
            "radio",
            "--name",
            "Contact",
            "--rect",
            "40,40,220,120",
            "--page",
            "3",
            "--option",
            "email=Email",
            "--option",
            "phone=Phone",
        ],
    );
    let combo = temp.path().join("combo.pdf");
    step(
        &radio,
        &combo,
        "add-form-field",
        &[
            "--kind",
            "combo",
            "--name",
            "Country",
            "--rect",
            "40,140,220,170",
            "--page",
            "3",
            "--option",
            "ch=Suisse & France",
            "--option",
            "be=Belgique",
        ],
    );
    let list = temp.path().join("list.pdf");
    step(
        &combo,
        &list,
        "add-form-field",
        &[
            "--kind",
            "list",
            "--name",
            "Languages",
            "--rect",
            "40,200,220,280",
            "--page",
            "3",
            "--option",
            "fr=French",
            "--option",
            "en=English",
            "--multi-select",
        ],
    );
    let filled = temp.path().join("filled.pdf");
    step(
        &list,
        &filled,
        "fill-form",
        &[
            "--set",
            "Contact=phone",
            "--set",
            "Country=ch",
            "--set",
            "Languages=fr;en",
        ],
    );
    assert_eq!(
        values(&filled),
        serde_json::json!({"Contact":"phone","Country":"ch","Languages":["fr","en"]})
    );
    let xfdf = temp.path().join("values.xfdf");
    step(&filled, &xfdf, "form-export", &[]);
    let roundtrip = temp.path().join("roundtrip.pdf");
    json(
        cli()
            .arg("fill-form")
            .arg(&list)
            .arg("--values")
            .arg(&xfdf)
            .arg("--out")
            .arg(&roundtrip)
            .output()
            .unwrap(),
    );
    assert_eq!(values(&roundtrip), values(&filled));
    let export_json = temp.path().join("values.json");
    step(&filled, &export_json, "form-export", &[]);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&std::fs::read(export_json).unwrap()).unwrap(),
        values(&filled)
    );

    let fdf = temp.path().join("values.fdf");
    let mut data = String::from("%FDF-1.2\n");
    let offset = data.len();
    data.push_str("1 0 obj\n<< /FDF << /Fields [<< /T (Country) /V (be) >>] >> >>\nendobj\n");
    let xref = data.len();
    data.push_str(&format!("xref\n0 2\n0000000000 65535 f \n{offset:010} 00000 n \ntrailer\n<< /Size 2 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"));
    std::fs::write(&fdf, data).unwrap();
    let imported = temp.path().join("fdf.pdf");
    json(
        cli()
            .arg("fill-form")
            .arg(&filled)
            .arg("--values")
            .arg(&fdf)
            .arg("--out")
            .arg(&imported)
            .output()
            .unwrap(),
    );
    assert_eq!(values(&imported)["Country"], "be");
    assert_eq!(std::fs::read(base).unwrap(), original);
    let invalid = temp.path().join("invalid-field.pdf");
    failed(
        cli()
            .arg("add-form-field")
            .arg(&list)
            .args([
                "--kind",
                "radio",
                "--name",
                "Bad",
                "--rect",
                "10,10,100,100",
                "--option",
                "same=One",
                "--option",
                "same=Two",
                "--out",
            ])
            .arg(&invalid)
            .output()
            .unwrap(),
        &invalid,
    );
}

#[test]
fn strict_or_oversized_import_fails_atomically_and_lenient_import_reports_skips() {
    let temp = tempfile::tempdir().unwrap();
    let input = fixture("acroform.pdf");
    let original = std::fs::read(&input).unwrap();
    for (name, bytes) in [
        ("unknown.json", br#"{"ville":"Changed","missing":"bad"}"#.to_vec()),
        ("unsafe.xfdf", br#"<!DOCTYPE xfdf [<!ENTITY e SYSTEM "file:///ignored">]><xfdf xmlns="http://ns.adobe.com/xfdf/"><fields><field name="ville"><value>&e;</value></field></fields></xfdf>"#.to_vec()),
        ("large.json", vec![b' '; sunipdf_core::form_data::IMPORT_LIMIT + 1]),
    ] {
        let values_file = temp.path().join(name);
        std::fs::write(&values_file, bytes).unwrap();
        let out = temp.path().join(format!("{name}.pdf"));
        failed(cli().arg("fill-form").arg(&input).arg("--values").arg(&values_file).arg("--out").arg(&out).output().unwrap(), &out);
    }
    let unknown = temp.path().join("unknown.json");
    let out = temp.path().join("lenient.pdf");
    let report = json(
        cli()
            .arg("fill-form")
            .arg(&input)
            .arg("--values")
            .arg(&unknown)
            .arg("--ignore-unknown")
            .arg("--out")
            .arg(&out)
            .output()
            .unwrap(),
    );
    assert_eq!(report["skipped"]["unknown"], serde_json::json!(["missing"]));
    assert_eq!(values(&out)["ville"], "Changed");
    assert_eq!(std::fs::read(input).unwrap(), original);
}

fn union(objects: &serde_json::Value) -> [f64; 4] {
    objects["objects"].as_array().unwrap().iter().fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |mut bounds, object| {
            for i in 0..4 {
                let value = object["bounds"][i].as_f64().unwrap();
                bounds[i] = if i < 2 {
                    bounds[i].min(value)
                } else {
                    bounds[i].max(value)
                };
            }
            bounds
        },
    )
}

#[test]
fn grouped_rotation_and_resize_reopen_with_expected_bounds_and_reject_partial_changes() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.pdf");
    std::fs::copy(fixture("two-pages.pdf"), &source).unwrap();
    let original = std::fs::read(&source).unwrap();
    let before = json(cli().arg("objects").arg(&source).output().unwrap());
    assert_eq!(before["objects"].as_array().unwrap().len(), 2);
    let old = union(&before);
    let rotated = temp.path().join("rotated.pdf");
    step(
        &source,
        &rotated,
        "rotate-objects",
        &["--objects", "0,1", "--degrees", "90"],
    );
    let after = union(&json(cli().arg("objects").arg(&rotated).output().unwrap()));
    assert!(((after[2] - after[0]) - (old[3] - old[1])).abs() < 0.1);
    assert!(((after[3] - after[1]) - (old[2] - old[0])).abs() < 0.1);
    let resized = temp.path().join("resized.pdf");
    step(
        &rotated,
        &resized,
        "resize-objects",
        &["--objects", "0,1", "--rect", "100,100,400,300"],
    );
    let bounds = union(&json(cli().arg("objects").arg(&resized).output().unwrap()));
    for (actual, expected) in bounds.into_iter().zip([100.0, 100.0, 400.0, 300.0]) {
        assert!((actual - expected).abs() < 0.1);
    }
    let png = temp.path().join("group.png");
    step(&resized, &png, "render", &["--page", "1", "--scale", "1"]);
    assert!(std::fs::metadata(png).unwrap().len() > 500);
    for (command, args) in [
        (
            "rotate-objects",
            vec!["--objects", "0,999", "--degrees", "90"],
        ),
        (
            "rotate-objects",
            vec!["--objects", "0,1", "--degrees", "NaN"],
        ),
        (
            "resize-objects",
            vec!["--objects", "0,1", "--rect", "100,100,0,200"],
        ),
    ] {
        let out = temp.path().join(format!("invalid-{command}.pdf"));
        failed(
            cli()
                .arg(command)
                .arg(&source)
                .args(args)
                .arg("--out")
                .arg(&out)
                .output()
                .unwrap(),
            &out,
        );
    }
    assert_eq!(std::fs::read(source).unwrap(), original);
}
