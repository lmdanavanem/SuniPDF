use lopdf::{dictionary, Document, Object, Stream};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"))
}

fn result(output: Output, code: i32) -> Value {
    assert_eq!(
        output.status.code(),
        Some(code),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}

fn refused(output: Output) -> Value {
    assert!(!output.status.success(), "unexpected success");
    assert!(output.stdout.is_empty());
    serde_json::from_slice(&output.stderr).unwrap()
}

fn pdf(text: &str) -> Vec<u8> {
    let mut document = Document::with_version("1.7");
    let pages = document.new_object_id();
    let font = document
        .add_object(dictionary! {"Type"=>"Font", "Subtype"=>"Type1", "BaseFont"=>"Helvetica"});
    let contents = document.add_object(Stream::new(
        dictionary! {},
        format!("BT /F1 12 Tf 50 200 Td ({text}) Tj ET").into_bytes(),
    ));
    let note = document.add_object(dictionary! {"Type"=>"Annot", "Subtype"=>"Text",
        "Rect"=>vec![10.into(),10.into(),30.into(),30.into()], "Contents"=>Object::string_literal("Retain this note")});
    let page = document.add_object(dictionary! {"Type"=>"Page", "Parent"=>pages,
    "MediaBox"=>vec![0.into(),0.into(),600.into(),800.into()], "Rotate"=>90,
    "Resources"=>dictionary! {"Font"=>dictionary! {"F1"=>font}},
    "Contents"=>contents, "Annots"=>vec![Object::Reference(note)]});
    document.objects.insert(
        pages,
        dictionary! {"Type"=>"Pages", "Count"=>1,
        "Kids"=>vec![Object::Reference(page)]}
        .into(),
    );
    let catalog = document.add_object(dictionary! {"Type"=>"Catalog", "Pages"=>pages});
    document.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).unwrap();
    bytes
}

struct Fixture {
    _dir: tempfile::TempDir,
    source: PathBuf,
    target: PathBuf,
    manifest: PathBuf,
    source_id: String,
    target_id: String,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("Index.pdf");
    let target = dir.path().join("Étage A.pdf");
    let manifest = dir.path().join("set.json");
    std::fs::write(&source, pdf("A-10 A-101 XA-10 A-10")).unwrap();
    std::fs::write(&target, pdf("Target drawing")).unwrap();
    result(
        cli()
            .args(["plan-set", "create"])
            .arg(&source)
            .arg(&target)
            .args(["--name", "References", "--out"])
            .arg(&manifest)
            .output()
            .unwrap(),
        0,
    );
    let mut set: Value = serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    let source_id = set["sheets"][0]["file_id"].as_str().unwrap().to_owned();
    let target_id = set["sheets"][1]["id"].as_str().unwrap().to_owned();
    set["sheets"][1]["number"] = json!("A-10");
    std::fs::write(&manifest, serde_json::to_vec_pretty(&set).unwrap()).unwrap();
    Fixture {
        _dir: dir,
        source,
        target,
        manifest,
        source_id,
        target_id,
    }
}

fn inspect(f: &Fixture) -> Command {
    let mut command = cli();
    command
        .args(["plan-links", "inspect"])
        .arg(&f.manifest)
        .args([
            "--source-file",
            &f.source_id,
            "--pages",
            "1",
            "--target-sheet",
            &f.target_id,
        ]);
    command
}

fn recipe(f: &Fixture) -> PathBuf {
    let report = result(inspect(f).output().unwrap(), 0);
    let report = &report["inspection"];
    let occurrences = report["occurrences"].as_array().unwrap();
    assert_eq!(
        occurrences.len(),
        2,
        "whole identifiers must not match longer words"
    );
    let selections: Vec<_> = occurrences
        .iter()
        .map(|item| {
            json!({
                "occurrence_id":item["id"], "fingerprint":item["fingerprint"],
                "target_sheet_id":f.target_id, "confirmed":true,
            })
        })
        .collect();
    let review = json!({"version":1, "manifest_sha256":report["manifest_sha256"],
        "source_sha256":report["source_sha256"], "scan":{
            "source_file_id":f.source_id, "pages":[1], "target_sheet_ids":[f.target_id]},
        "selections":selections});
    let path = f._dir.path().join("review.json");
    std::fs::write(&path, serde_json::to_vec_pretty(&review).unwrap()).unwrap();
    path
}

fn apply(f: &Fixture, review: &Path, out: &Path) -> Command {
    let mut command = cli();
    command
        .args(["plan-links", "apply"])
        .arg(&f.manifest)
        .arg("--review")
        .arg(review)
        .arg("--out")
        .arg(out);
    command
}

#[test]
fn inspected_links_preview_save_reopen_preserve_source_and_refuse_overwrite() {
    let f = fixture();
    let protected: Vec<_> = [&f.source, &f.target, &f.manifest]
        .into_iter()
        .map(|p| (p, std::fs::read(p).unwrap()))
        .collect();
    let review = recipe(&f);
    let png = f._dir.path().join("preview.png");
    let preview = result(
        cli()
            .args(["plan-links", "preview"])
            .arg(&f.manifest)
            .arg("--review")
            .arg(&review)
            .arg("--preview-out")
            .arg(&png)
            .args(["--preview-long-side", "320"])
            .output()
            .unwrap(),
        0,
    );
    assert_eq!(preview["published"], false);
    let image = image::open(&png).unwrap();
    assert_eq!(image.width().max(image.height()), 320);
    assert_eq!(preview["preview_image"]["kind"], "source_candidate");
    let target_png = f._dir.path().join("destination.png");
    let target_preview = result(
        cli()
            .args(["plan-links", "preview"])
            .arg(&f.manifest)
            .arg("--review")
            .arg(&review)
            .arg("--preview-out")
            .arg(&target_png)
            .args([
                "--preview-target-sheet",
                &f.target_id,
                "--preview-long-side",
                "320",
            ])
            .output()
            .unwrap(),
        0,
    );
    assert_eq!(target_preview["preview_image"]["kind"], "destination");
    assert_eq!(
        target_preview["preview_image"]["target_sheet_id"],
        f.target_id
    );
    assert_ne!(
        std::fs::read(&png).unwrap(),
        std::fs::read(&target_png).unwrap()
    );
    let output = f._dir.path().join("linked.pdf");
    let receipt = result(apply(&f, &review, &output).output().unwrap(), 0);
    assert_eq!(receipt["published"], true);
    let document = Document::load(&output).unwrap();
    let page = document.get_pages()[&1];
    let annotations = document
        .get_dictionary(page)
        .unwrap()
        .get(b"Annots")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(annotations.len(), 3);
    let mut links = 0;
    for annotation in annotations {
        let annotation = document
            .dereference(annotation)
            .unwrap()
            .1
            .as_dict()
            .unwrap();
        if annotation.get(b"Subtype").unwrap().as_name().unwrap() == b"Link" {
            links += 1;
            let action = annotation.get(b"A").unwrap().as_dict().unwrap();
            assert_eq!(action.get(b"S").unwrap().as_name().unwrap(), b"GoToR");
            assert!(action.has(b"SuniPDFPlanTarget"));
            assert!(!action.has(b"Next"));
        }
    }
    assert_eq!(links, 2);
    let saved = std::fs::read(&output).unwrap();
    refused(
        apply(&f, &review, &output)
            .arg("--overwrite")
            .output()
            .unwrap(),
    );
    refused(
        apply(&f, &review, &f.source)
            .arg("--overwrite")
            .output()
            .unwrap(),
    );
    assert_eq!(std::fs::read(&output).unwrap(), saved);
    for (path, bytes) in protected {
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn changed_target_and_unconfirmed_review_cannot_create_a_copy() {
    let f = fixture();
    let review = recipe(&f);
    let output = f._dir.path().join("refused.pdf");
    let original = std::fs::read(&review).unwrap();
    let mut changed: Value = serde_json::from_slice(&original).unwrap();
    changed["selections"][0]["confirmed"] = json!(false);
    std::fs::write(&review, serde_json::to_vec(&changed).unwrap()).unwrap();
    refused(apply(&f, &review, &output).output().unwrap());
    assert!(!output.exists());
    std::fs::write(&review, original).unwrap();
    let mut target = std::fs::read(&f.target).unwrap();
    target.extend_from_slice(b"\n% changed after review\n");
    std::fs::write(&f.target, target).unwrap();
    refused(apply(&f, &review, &output).output().unwrap());
    assert!(!output.exists());
}

#[test]
fn password_sources_are_explicit_and_errors_do_not_echo_values() {
    let f = fixture();
    let environment = "SUNIPDF_LINKS_CLI_TEST_PASSWORD";
    let value = format!("{}-invalid", f.source_id);
    let spec = format!("{}={environment}", f.source_id);
    let report = refused(
        inspect(&f)
            .args(["--file-password-env", &spec, "--file-password-env", &spec])
            .env(environment, &value)
            .output()
            .unwrap(),
    );
    assert!(!report.to_string().contains(&value));
    let report = refused(
        inspect(&f)
            .args(["--file-password-env", &spec])
            .env_remove(environment)
            .output()
            .unwrap(),
    );
    assert!(!report.to_string().contains(environment));
}

#[test]
fn batch_preflights_every_output_and_keeps_mutual_links_pointing_to_originals() {
    let dir = tempfile::tempdir().unwrap();
    let originals = [dir.path().join("A.pdf"), dir.path().join("B.pdf")];
    std::fs::write(&originals[0], pdf("A-20")).unwrap();
    std::fs::write(&originals[1], pdf("A-10")).unwrap();
    let manifest = dir.path().join("set.json");
    result(
        cli()
            .args(["plan-set", "create"])
            .args(&originals)
            .args(["--name", "Mutual references", "--out"])
            .arg(&manifest)
            .output()
            .unwrap(),
        0,
    );
    let mut set: Value = serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    set["sheets"][0]["number"] = json!("A-10");
    set["sheets"][1]["number"] = json!("A-20");
    std::fs::write(&manifest, serde_json::to_vec_pretty(&set).unwrap()).unwrap();
    let original_bytes: Vec<_> = originals
        .iter()
        .map(|p| std::fs::read(p).unwrap())
        .collect();
    let sources: Vec<_> = (0..2)
        .map(|i| {
            json!({
                "source_file_id":set["sheets"][i]["file_id"], "pages":[1],
                "target_sheet_ids":[set["sheets"][1-i]["id"]]
            })
        })
        .collect();
    let request = dir.path().join("request.json");
    std::fs::write(
        &request,
        serde_json::to_vec(&json!({"sources":sources})).unwrap(),
    )
    .unwrap();
    let inspection = result(
        cli()
            .args(["plan-links", "batch-inspect"])
            .arg(&manifest)
            .arg("--request")
            .arg(&request)
            .output()
            .unwrap(),
        0,
    );
    let reports = inspection["inspection"]["files"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    let files: Vec<_> = reports
        .iter()
        .enumerate()
        .map(|(i, report)| {
            assert_eq!(report["occurrences"].as_array().unwrap().len(), 1);
            let occurrence = &report["occurrences"][0];
            json!({"output_name":format!("linked-{i}.pdf"), "review":{
            "version":1,"manifest_sha256":report["manifest_sha256"],
            "source_sha256":report["source_sha256"],"scan":sources[i],
            "selections":[{"occurrence_id":occurrence["id"],"fingerprint":occurrence["fingerprint"],
                "target_sheet_id":set["sheets"][1-i]["id"],"confirmed":true}]}})
        })
        .collect();
    let recipe = json!({"version":1,"manifest_sha256":reports[0]["manifest_sha256"],"files":files});
    let review = dir.path().join("review.json");
    std::fs::write(&review, serde_json::to_vec(&recipe).unwrap()).unwrap();
    let output_a = dir.path().join("linked-0.pdf");
    let output_b = dir.path().join("linked-1.pdf");
    std::fs::write(&output_b, b"Keep existing bytes").unwrap();
    refused(
        cli()
            .args(["plan-links", "batch-apply"])
            .arg(&manifest)
            .arg("--review")
            .arg(&review)
            .arg("--overwrite")
            .output()
            .unwrap(),
    );
    assert!(
        !output_a.exists(),
        "the second output must be checked before the first is written"
    );
    assert_eq!(std::fs::read(&output_b).unwrap(), b"Keep existing bytes");
    std::fs::remove_file(&output_b).unwrap();
    let preview = dir.path().join("batch-preview.png");
    let report = result(
        cli()
            .args(["plan-links", "batch-preview"])
            .arg(&manifest)
            .arg("--review")
            .arg(&review)
            .arg("--source-file")
            .arg(set["sheets"][0]["file_id"].as_str().unwrap())
            .arg("--preview-out")
            .arg(&preview)
            .args(["--preview-long-side", "320"])
            .output()
            .unwrap(),
        0,
    );
    assert_eq!(report["published"], false);
    let image = image::open(preview).unwrap();
    assert_eq!(image.width().max(image.height()), 320);
    assert!(!output_a.exists() && !output_b.exists());
    let applied = result(
        cli()
            .args(["plan-links", "batch-apply"])
            .arg(&manifest)
            .arg("--review")
            .arg(&review)
            .output()
            .unwrap(),
        0,
    );
    assert_eq!(applied["complete"], true);
    assert_eq!(applied["atomicity"], "per_file");
    let receipts = applied["outcome"]["completed"].as_array().unwrap();
    assert_eq!(receipts.len(), 2);
    for (i, output) in [output_a, output_b].iter().enumerate() {
        let doc = Document::load(output).unwrap();
        let page = doc.get_dictionary(doc.get_pages()[&1]).unwrap();
        let annotations = page.get(b"Annots").unwrap().as_array().unwrap();
        assert_eq!(annotations.len(), 2, "original note plus one reviewed link");
        let link = annotations
            .iter()
            .map(|v| doc.dereference(v).unwrap().1.as_dict().unwrap())
            .find(|a| a.get(b"Subtype").unwrap().as_name().unwrap() == b"Link")
            .unwrap();
        let action = link.get(b"A").unwrap().as_dict().unwrap();
        let target: Value =
            serde_json::from_slice(action.get(b"SuniPDFPlanTarget").unwrap().as_str().unwrap())
                .unwrap();
        assert_eq!(
            target["relative_path"],
            if i == 0 { "B.pdf" } else { "A.pdf" }
        );
        assert_eq!(std::fs::read(&originals[i]).unwrap(), original_bytes[i]);
    }
    let mut invalid = recipe;
    invalid["files"][1]["output_name"] = json!("../escape.pdf");
    std::fs::write(&review, serde_json::to_vec(&invalid).unwrap()).unwrap();
    refused(
        cli()
            .args(["plan-links", "batch-apply"])
            .arg(&manifest)
            .arg("--review")
            .arg(&review)
            .output()
            .unwrap(),
    );
}
