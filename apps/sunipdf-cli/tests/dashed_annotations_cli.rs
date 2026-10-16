//! Dashed shape annotations round-trip through the binary, standard PDF data and native rendering.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use sunipdf_core::{security, Engine, Protection};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/sunipdf-core/tests/fixtures")
        .join(name)
}

fn cli(action: &str, source: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
    command.arg(action).arg(source);
    command
}

fn annotate(source: &Path, out: &Path, kind: &str) -> Command {
    let mut command = cli("annotate", source);
    command
        .args([
            "--kind",
            kind,
            "--rect",
            "100,100,220,180",
            "--color",
            "FF0000",
            "--width",
            "2",
            "--out",
        ])
        .arg(out);
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

fn first_annotation_id(pdf: &lopdf::Document) -> lopdf::ObjectId {
    let page = pdf.get_pages()[&1];
    pdf.get_object(page)
        .unwrap()
        .as_dict()
        .unwrap()
        .get(b"Annots")
        .unwrap()
        .as_array()
        .unwrap()[0]
        .as_reference()
        .unwrap()
}

fn appearance_id(pdf: &lopdf::Document, annotation: lopdf::ObjectId) -> lopdf::ObjectId {
    pdf.get_object(annotation)
        .unwrap()
        .as_dict()
        .unwrap()
        .get(b"AP")
        .unwrap()
        .as_dict()
        .unwrap()
        .get(b"N")
        .unwrap()
        .as_reference()
        .unwrap()
}

fn assert_standard_dash(path: &Path, password: Option<&str>, expected: &[f32]) {
    let bytes = std::fs::read(path).unwrap();
    let pdf = lopdf::Document::load_mem_with_options(
        &bytes,
        lopdf::LoadOptions::with_password(password.unwrap_or("")),
    )
    .unwrap();
    let id = first_annotation_id(&pdf);
    let annotation = pdf.get_object(id).unwrap().as_dict().unwrap();
    let border = annotation.get(b"BS").unwrap().as_dict().unwrap();
    assert_eq!(border.get(b"S").unwrap().as_name().unwrap(), b"D");
    let pattern: Vec<_> = border
        .get(b"D")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_float().unwrap())
        .collect();
    assert_eq!(pattern, expected);
    let stream = pdf
        .get_object(appearance_id(&pdf, id))
        .unwrap()
        .as_stream()
        .unwrap();
    let content = stream
        .decompressed_content()
        .unwrap_or_else(|_| stream.content.clone());
    let operations = lopdf::content::Content::decode(&content)
        .unwrap()
        .operations;
    let dash = operations.iter().find(|op| op.operator == "d").unwrap();
    let actual: Vec<_> = dash.operands[0]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_float().unwrap())
        .collect();
    assert_eq!(actual, expected);
    assert_eq!(dash.operands[1].as_float().unwrap(), 0.0);
}

fn red(pixel: &[u8]) -> bool {
    pixel[0] > 180 && pixel[1] < 80 && pixel[2] < 80
}

#[test]
fn shapes_render_real_gaps_and_preserve_custom_patterns_and_encrypted_roles() {
    // All native checks in this process share one PDFium engine.
    let engine = Engine::new().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let source = fixture("two-pages.pdf");
    let original = std::fs::read(&source).unwrap();
    let before = engine.open(&source, None).unwrap();
    for (kind, subtype) in [
        ("square", "Square"),
        ("circle", "Circle"),
        ("line", "Line"),
        ("arrow", "Line"),
    ] {
        let dashed = temp.path().join(format!("{kind}-dashed.pdf"));
        assert_eq!(
            success(
                annotate(&source, &dashed, kind)
                    .arg("--dashed")
                    .output()
                    .unwrap()
            )["index"],
            0
        );
        let listed = success(cli("annotations", &dashed).output().unwrap());
        let annotation = &listed["pages"][0]["annotations"][0];
        assert_eq!(annotation["dash_pattern"], serde_json::json!([6.0, 3.0]));
        assert_eq!(annotation["subtype"], subtype);
        assert_eq!(annotation["arrow"], kind == "arrow");
        assert_standard_dash(&dashed, None, &[6.0, 3.0]);
        let after = engine.open(&dashed, None).unwrap();
        assert_eq!(after.page_count(), before.page_count());
        for page in 0..before.page_count() {
            assert_eq!(
                after.page_size(page).unwrap(),
                before.page_size(page).unwrap()
            );
            assert_eq!(
                after.page_text(page).unwrap(),
                before.page_text(page).unwrap()
            );
        }
        assert_eq!(
            after.render_page(1, 0.5).unwrap().rgba,
            before.render_page(1, 0.5).unwrap().rgba
        );
        let dashed_pixels = after.render_page(0, 2.0).unwrap().rgba;
        assert_eq!(
            dashed_pixels,
            engine
                .open(&dashed, None)
                .unwrap()
                .render_page(0, 2.0)
                .unwrap()
                .rgba
        );
        let solid = temp.path().join(format!("{kind}-solid.pdf"));
        success(annotate(&source, &solid, kind).output().unwrap());
        assert_eq!(
            success(cli("annotations", &solid).output().unwrap())["pages"][0]["annotations"][0]
                ["dash_pattern"],
            serde_json::json!([])
        );
        let solid_pixels = engine
            .open(&solid, None)
            .unwrap()
            .render_page(0, 2.0)
            .unwrap()
            .rgba;
        assert_eq!(dashed_pixels.len() % 4, 0);
        assert_eq!(solid_pixels.len(), dashed_pixels.len());
        let red_dashed = dashed_pixels.chunks(4).filter(|pixel| red(pixel)).count();
        let red_solid = solid_pixels.chunks(4).filter(|pixel| red(pixel)).count();
        assert!(
            red_dashed > 50 && red_dashed * 10 < red_solid * 9,
            "{kind}: {red_dashed}/{red_solid}"
        );
        let real_gaps = solid_pixels
            .chunks(4)
            .zip(dashed_pixels.chunks(4))
            .filter(|(solid, dashed)| {
                red(solid) && dashed[..3].iter().all(|channel| *channel > 245)
            })
            .count();
        assert!(real_gaps > 20, "{kind}: no rendered gaps");
    }

    let dashed = temp.path().join("square-dashed.pdf");
    let mut pdf = lopdf::Document::load(&dashed).unwrap();
    let id = first_annotation_id(&pdf);
    let ap_id = appearance_id(&pdf, id);
    let annotation = pdf.get_object_mut(id).unwrap().as_dict_mut().unwrap();
    annotation.remove(b"NM");
    annotation
        .get_mut(b"BS")
        .unwrap()
        .as_dict_mut()
        .unwrap()
        .set("D", vec![2.into(), 1.into(), 4.into()]);
    let stream = pdf.get_object_mut(ap_id).unwrap().as_stream_mut().unwrap();
    let content = stream
        .decompressed_content()
        .unwrap_or_else(|_| stream.content.clone());
    stream.dict.remove(b"Filter");
    stream.set_content(
        String::from_utf8(content)
            .unwrap()
            .replace("[6 3] 0 d", "[2 1 4] 0 d")
            .into_bytes(),
    );
    let custom = temp.path().join("external-custom.pdf");
    pdf.save(&custom).unwrap();
    let custom_bytes = std::fs::read(&custom).unwrap();
    let custom_doc = engine.open(&custom, None).unwrap();
    let custom_render = custom_doc.render_page(0, 1.0).unwrap().rgba;
    let listed = success(cli("annotations", &custom).output().unwrap());
    assert_eq!(
        listed["pages"][0]["annotations"][0]["dash_pattern"],
        serde_json::json!([2.0, 1.0, 4.0])
    );
    assert_eq!(listed["pages"][0]["annotations"][0]["ours"], false);
    let copied = temp.path().join("custom-metadata.pdf");
    success(
        cli("set-metadata", &custom)
            .args(["--title", "Custom dash preserved", "--out"])
            .arg(&copied)
            .output()
            .unwrap(),
    );
    assert_standard_dash(&copied, None, &[2.0, 1.0, 4.0]);
    assert_eq!(
        engine
            .open(&copied, None)
            .unwrap()
            .render_page(0, 1.0)
            .unwrap()
            .rgba,
        custom_render
    );
    assert_eq!(std::fs::read(&custom).unwrap(), custom_bytes);

    let expected_render = engine
        .open(&temp.path().join("line-dashed.pdf"), None)
        .unwrap()
        .render_page(0, 1.0)
        .unwrap()
        .rgba;
    for aes256 in [false, true] {
        let protected = security::protect(
            &original,
            None,
            &Protection {
                user_password: "dash-user".into(),
                owner_password: "dash-owner".into(),
                allow_print: false,
                allow_copy: false,
                allow_annotate: false,
                aes256,
                ..Default::default()
            },
        )
        .unwrap();
        let protected_source = temp.path().join(format!("protected-{aes256}.pdf"));
        std::fs::write(&protected_source, &protected).unwrap();
        for opening_password in ["dash-user", "dash-owner"] {
            let out = temp
                .path()
                .join(format!("dashed-{aes256}-{opening_password}.pdf"));
            success(
                annotate(&protected_source, &out, "line")
                    .args(["--dashed", "--password", opening_password])
                    .output()
                    .unwrap(),
            );
            assert!(engine.open(&out, None).is_err());
            for password in ["dash-user", "dash-owner"] {
                assert_standard_dash(&out, Some(password), &[6.0, 3.0]);
                let after = engine.open(&out, Some(password)).unwrap();
                assert_eq!(
                    after.annotation_details(0).unwrap()[0].dash_pattern,
                    [6.0, 3.0]
                );
                let properties = after.properties();
                assert!(properties.encrypted && properties.can_modify);
                assert!(!properties.can_print && !properties.can_copy && !properties.can_annotate);
                assert_eq!(after.render_page(0, 1.0).unwrap().rgba, expected_render);
                for page in 0..before.page_count() {
                    assert_eq!(
                        after.page_size(page).unwrap(),
                        before.page_size(page).unwrap()
                    );
                    assert_eq!(
                        after.page_text(page).unwrap(),
                        before.page_text(page).unwrap()
                    );
                }
                let listed = success(
                    cli("annotations", &out)
                        .args(["--password", password])
                        .output()
                        .unwrap(),
                );
                assert_eq!(
                    listed["pages"][0]["annotations"][0]["dash_pattern"],
                    serde_json::json!([6.0, 3.0])
                );
            }
            assert_eq!(std::fs::read(&protected_source).unwrap(), protected);
        }
    }
    assert_eq!(std::fs::read(&source).unwrap(), original);
}

#[test]
fn unsupported_dash_kinds_and_invalid_shapes_preserve_existing_output() {
    let temp = tempfile::tempdir().unwrap();
    let source = fixture("two-pages.pdf");
    let original = std::fs::read(&source).unwrap();
    for kind in ["ink", "freetext", "link", "unknown"] {
        for overwrite in [false, true] {
            let out = temp.path().join(format!("{kind}-{overwrite}.pdf"));
            if overwrite {
                std::fs::write(&out, &original).unwrap();
            }
            let input = if overwrite { &out } else { &source };
            let mut command = annotate(input, &out, kind);
            command.arg("--dashed");
            if overwrite {
                command.arg("--overwrite");
            }
            let output = command.output().unwrap();
            assert_eq!(output.status.code(), Some(1));
            let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
            assert_eq!(error["error"], "other");
            assert!(error["message"]
                .as_str()
                .unwrap()
                .contains("--dashed only supports"));
            if overwrite {
                assert_eq!(std::fs::read(&out).unwrap(), original);
            } else {
                assert!(!out.exists());
            }
        }
    }
    let out = temp.path().join("invalid-shape.pdf");
    let sentinel = b"existing output must survive";
    for args in [
        vec!["--rect", "1,2,3"],
        vec!["--rect", "1,2,NaN,4"],
        vec!["--width", "-1"],
        vec!["--page", "0"],
    ] {
        std::fs::write(&out, sentinel).unwrap();
        let output = cli("annotate", &source)
            .args(["--kind", "square", "--dashed", "--out"])
            .arg(&out)
            .args(args)
            .arg("--overwrite")
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert_eq!(std::fs::read(&out).unwrap(), sentinel);
    }
    std::fs::write(&out, sentinel).unwrap();
    assert_eq!(
        annotate(&source, &out, "square")
            .arg("--dashed")
            .output()
            .unwrap()
            .status
            .code(),
        Some(6)
    );
    assert_eq!(std::fs::read(&out).unwrap(), sentinel);
    assert_eq!(std::fs::read(&source).unwrap(), original);
}
