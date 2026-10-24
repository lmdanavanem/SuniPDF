use lopdf::{dictionary, Document, Object, Stream};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"))
}

fn ok(output: Output, code: i32) -> Value {
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
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    serde_json::from_slice(&output.stderr).unwrap()
}

fn pdf(changed: bool) -> Vec<u8> {
    let mut document = Document::with_version("1.7");
    let pages = document.new_object_id();
    let content = if changed {
        b"0 g 70 80 35 25 re f 150 200 45 30 re f 370 510 30 40 re f".as_slice()
    } else {
        b"0 g 70 80 35 25 re f".as_slice()
    };
    let stream = document.add_object(Stream::new(dictionary! {}, content.to_vec()));
    let note = document.add_object(dictionary! {"Type"=>"Annot", "Subtype"=>"Text",
        "Rect"=>vec![25.into(),35.into(),45.into(),55.into()], "Contents"=>Object::string_literal("Existing note")});
    let page = document.add_object(dictionary! {"Type"=>"Page", "Parent"=>pages,
    "MediaBox"=>vec![0.into(),0.into(),600.into(),800.into()],
    "CropBox"=>vec![20.into(),30.into(),580.into(),770.into()], "Rotate"=>90,
    "Resources"=>dictionary!{}, "Contents"=>stream, "Annots"=>vec![Object::Reference(note)]});
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
    directory: tempfile::TempDir,
    old: PathBuf,
    new: PathBuf,
}

fn fixture() -> Fixture {
    let directory = tempfile::tempdir().unwrap();
    let old = directory.path().join("old.pdf");
    let new = directory.path().join("new.pdf");
    std::fs::write(&old, pdf(false)).unwrap();
    std::fs::write(&new, pdf(true)).unwrap();
    Fixture {
        directory,
        old,
        new,
    }
}

fn inspect(old: &Path, new: &Path) -> Value {
    ok(
        cli()
            .args(["revision-outlines", "inspect"])
            .arg(old)
            .arg(new)
            .args([
                "--long-side",
                "400",
                "--min-component-pixels",
                "4",
                "--merge-gap-pixels",
                "0",
            ])
            .output()
            .unwrap(),
        0,
    )
}

fn review(fixture: &Fixture, all_noise: bool) -> PathBuf {
    let value = inspect(&fixture.old, &fixture.new);
    let inspection = &value["inspection"];
    assert_eq!(value["complete"], true);
    let regions = inspection["regions"].as_array().unwrap();
    assert_eq!(
        regions.len(),
        2,
        "the two separated added rectangles should yield two proposals"
    );
    let selections: Vec<_> = regions.iter().enumerate().map(|(index, region)| json!({
        "id":region["id"], "fingerprint":region["fingerprint"],
        "classification":if all_noise || index > 0 {"rendering_noise"} else {"content_change"},
        "confirmed":true,
    })).collect();
    let value = json!({"version":1,"old_sha256":inspection["old_sha256"],
        "new_sha256":inspection["new_sha256"],"options":inspection["options"],"selections":selections});
    let path = fixture.directory.path().join(if all_noise {
        "noise-review.json"
    } else {
        "review.json"
    });
    std::fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    path
}

fn command(fixture: &Fixture, operation: &str, review: &Path) -> Command {
    let mut result = cli();
    result
        .args(["revision-outlines", operation])
        .arg(&fixture.old)
        .arg(&fixture.new)
        .arg("--review")
        .arg(review);
    result
}

#[test]
fn identical_pages_create_no_proposals_and_reviewed_changes_round_trip_in_a_new_copy() {
    let fixture = fixture();
    let originals = [
        std::fs::read(&fixture.old).unwrap(),
        std::fs::read(&fixture.new).unwrap(),
    ];
    let identical_copy = fixture.directory.path().join("identical.pdf");
    std::fs::copy(&fixture.old, &identical_copy).unwrap();
    let identical = inspect(&fixture.old, &identical_copy);
    assert_eq!(identical["inspection"]["changed_pixels"], 0);
    assert!(identical["inspection"]["regions"]
        .as_array()
        .unwrap()
        .is_empty());
    let review = review(&fixture, false);
    let png = fixture.directory.path().join("preview.png");
    let preview = ok(
        command(&fixture, "preview", &review)
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
    let output = fixture.directory.path().join("reviewed.pdf");
    let applied = ok(
        command(&fixture, "apply", &review)
            .arg("--out")
            .arg(&output)
            .output()
            .unwrap(),
        0,
    );
    assert_eq!(applied["published"], true);
    let document = Document::load(&output).unwrap();
    let page = document.get_pages()[&1];
    assert_eq!(
        document
            .get_dictionary(page)
            .unwrap()
            .get(b"Rotate")
            .unwrap()
            .as_i64()
            .unwrap(),
        90
    );
    let annots = document
        .get_dictionary(page)
        .unwrap()
        .get(b"Annots")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(
        annots.len(),
        2,
        "one retained note and one accepted outline; noise creates nothing"
    );
    let original_new = Document::load(&fixture.new).unwrap();
    assert_eq!(
        document.get_page_content(page).unwrap(),
        original_new
            .get_page_content(original_new.get_pages()[&1])
            .unwrap()
    );
    ok(cli().arg("annotations").arg(&output).output().unwrap(), 0);
    let saved = std::fs::read(&output).unwrap();
    refused(
        command(&fixture, "apply", &review)
            .arg("--out")
            .arg(&output)
            .arg("--overwrite")
            .output()
            .unwrap(),
    );
    assert_eq!(saved, std::fs::read(&output).unwrap());
    assert_eq!(originals[0], std::fs::read(&fixture.old).unwrap());
    assert_eq!(originals[1], std::fs::read(&fixture.new).unwrap());
}

#[test]
fn noise_only_decisions_remain_reportable_without_publishing_an_empty_pdf() {
    let fixture = fixture();
    let review = review(&fixture, true);
    let result = ok(command(&fixture, "preview", &review).output().unwrap(), 0);
    assert_eq!(result["can_publish"], false);
    let output = fixture.directory.path().join("noise-only.pdf");
    refused(
        command(&fixture, "apply", &review)
            .arg("--out")
            .arg(&output)
            .output()
            .unwrap(),
    );
    assert!(!output.exists());
}

#[test]
fn unconfirmed_stale_and_invalid_alignment_inputs_publish_no_pdf() {
    let fixture = fixture();
    let path = review(&fixture, false);
    let original = std::fs::read(&path).unwrap();
    let mut unconfirmed: Value = serde_json::from_slice(&original).unwrap();
    unconfirmed["selections"][0]["confirmed"] = json!(false);
    std::fs::write(&path, serde_json::to_vec(&unconfirmed).unwrap()).unwrap();
    let output = fixture.directory.path().join("must-not-exist.pdf");
    refused(
        command(&fixture, "apply", &path)
            .arg("--out")
            .arg(&output)
            .output()
            .unwrap(),
    );
    assert!(!output.exists());
    std::fs::write(&path, original).unwrap();
    let mut changed = std::fs::read(&fixture.new).unwrap();
    changed.extend_from_slice(b"\n% Later revision\n");
    std::fs::write(&fixture.new, changed).unwrap();
    refused(
        command(&fixture, "apply", &path)
            .arg("--out")
            .arg(&output)
            .output()
            .unwrap(),
    );
    assert!(!output.exists());
    for args in [
        vec!["--old-page", "0"],
        vec!["--scale", "NaN"],
        vec!["--channel-tolerance", "255"],
        vec!["--merge-gap-pixels", "17"],
        vec!["--old-points", "0,0 0,0"],
    ] {
        let invalid = cli()
            .args(["revision-outlines", "inspect"])
            .arg(&fixture.old)
            .arg(&fixture.new)
            .args(args)
            .output()
            .unwrap();
        assert!(!invalid.status.success());
    }
    let invalid = command(&fixture, "preview", &path)
        .args(["--password-stdin", "--new-password-stdin"])
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert!(refused(invalid)
        .to_string()
        .contains("only one PDF password"));
}

fn page_pair_pdf(page_count: u32, selected_page: u32, changed: bool) -> Vec<u8> {
    let mut document = Document::with_version("1.7");
    let pages = document.new_object_id();
    let mut children = Vec::new();
    for number in 1..=page_count {
        let content = if number != selected_page {
            format!("0 g {} 50 20 20 re f", number * 30)
        } else if changed {
            "0 g 120 100 40 40 re f 300 500 40 40 re f".into()
        } else {
            "0 g 80 100 40 40 re f".into()
        };
        let stream = document.add_object(Stream::new(dictionary! {}, content.into_bytes()));
        let page = document.add_object(dictionary! {"Type"=>"Page", "Parent"=>pages,
        "MediaBox"=>vec![0.into(),0.into(),600.into(),800.into()],
        "Resources"=>dictionary!{}, "Contents"=>stream});
        children.push(Object::Reference(page));
    }
    document.objects.insert(
        pages,
        dictionary! {"Type"=>"Pages", "Count"=>page_count,
        "Kids"=>children}
        .into(),
    );
    let catalog = document.add_object(dictionary! {"Type"=>"Catalog", "Pages"=>pages});
    document.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).unwrap();
    bytes
}

#[test]
fn distinct_protected_sources_keep_physical_page_mapping_alignment_and_encryption() {
    let directory = tempfile::tempdir().unwrap();
    let old_plain = directory.path().join("old-plain.pdf");
    let new_plain = directory.path().join("new-plain.pdf");
    std::fs::write(&old_plain, page_pair_pdf(2, 2, false)).unwrap();
    std::fs::write(&new_plain, page_pair_pdf(3, 3, true)).unwrap();
    let old = directory.path().join("old-protected.pdf");
    let new = directory.path().join("new-protected.pdf");
    let old_secret = format!("old-owner-{}", std::process::id());
    let new_secret = format!("new-owner-{}", std::process::id());
    for (source, output, owner) in [
        (&old_plain, &old, &old_secret),
        (&new_plain, &new, &new_secret),
    ] {
        let opening = format!("opening-{owner}");
        ok(
            cli()
                .arg("protect")
                .arg(source)
                .args([
                    "--user-password-env",
                    "SUNIPDF_OUTLINE_OPENING",
                    "--owner-password-env",
                    "SUNIPDF_OUTLINE_OWNER",
                    "--out",
                ])
                .arg(output)
                .env("SUNIPDF_OUTLINE_OPENING", opening)
                .env("SUNIPDF_OUTLINE_OWNER", owner)
                .output()
                .unwrap(),
            0,
        );
    }
    let protected_originals = [std::fs::read(&old).unwrap(), std::fs::read(&new).unwrap()];
    let protected_command = |operation: &str| {
        let mut command = cli();
        command
            .args(["revision-outlines", operation])
            .arg(&old)
            .arg(&new)
            .args([
                "--password-env",
                "SUNIPDF_OUTLINE_OLD",
                "--new-password-env",
                "SUNIPDF_OUTLINE_NEW",
            ])
            .env("SUNIPDF_OUTLINE_OLD", &old_secret)
            .env("SUNIPDF_OUTLINE_NEW", &new_secret);
        assert!(!command
            .get_args()
            .any(|arg| arg == old_secret.as_str() || arg == new_secret.as_str()));
        command
    };
    let inspected = ok(
        protected_command("inspect")
            .args([
                "--old-page",
                "2",
                "--new-page",
                "3",
                "--translate",
                "40,0",
                "--long-side",
                "400",
                "--merge-gap-pixels",
                "0",
            ])
            .output()
            .unwrap(),
        2,
    );
    assert_eq!(
        inspected["complete"], false,
        "translated page edges remain outside comparison coverage"
    );
    let inspection = &inspected["inspection"];
    assert_eq!(inspection["old_geometry"]["page"], 2);
    assert_eq!(inspection["new_geometry"]["page"], 3);
    assert_eq!(inspection["transform"]["translation"], json!([40.0, 0.0]));
    assert!(inspection["unmapped_pixels"].as_u64().unwrap() > 0);
    assert!(inspection["unmapped_old_pixels"].as_u64().unwrap() > 0);
    let regions = inspection["regions"].as_array().unwrap();
    assert_eq!(
        regions.len(),
        1,
        "aligned retained content must not become an extra proposal"
    );
    let region = &regions[0];
    let review = json!({"version":1, "old_sha256":inspection["old_sha256"],
        "new_sha256":inspection["new_sha256"], "options":inspection["options"],
        "selections":[{"id":region["id"], "fingerprint":region["fingerprint"],
            "classification":"uncertain", "confirmed":true}]});
    let review_path = directory.path().join("protected-review.json");
    std::fs::write(&review_path, serde_json::to_vec(&review).unwrap()).unwrap();
    let output = directory.path().join("reviewed-protected.pdf");
    let applied = ok(
        protected_command("apply")
            .arg("--review")
            .arg(&review_path)
            .arg("--out")
            .arg(&output)
            .output()
            .unwrap(),
        2,
    );
    assert_eq!(applied["published"], true);
    assert_eq!(applied["complete"], false);
    assert!(!applied.to_string().contains(&old_secret));
    assert!(!applied.to_string().contains(&new_secret));
    let mut saved = Document::load(&output).unwrap();
    assert!(saved.is_encrypted());
    assert!(saved.decrypt(&old_secret).is_err());
    let mut saved = Document::load(&output).unwrap();
    saved.decrypt(&new_secret).unwrap();
    let original = Document::load(&new_plain).unwrap();
    let saved_pages = saved.get_pages();
    let original_pages = original.get_pages();
    assert_eq!(saved_pages.len(), 3);
    for number in 1..=3 {
        let page = saved_pages[&number];
        assert_eq!(
            saved.get_page_content(page).unwrap(),
            original.get_page_content(original_pages[&number]).unwrap()
        );
        let annotations = saved.get_dictionary(page).unwrap().get(b"Annots").ok();
        if number == 3 {
            assert_eq!(annotations.unwrap().as_array().unwrap().len(), 1);
        } else {
            assert!(annotations.is_none());
        }
    }
    assert_eq!(protected_originals[0], std::fs::read(&old).unwrap());
    assert_eq!(protected_originals[1], std::fs::read(&new).unwrap());
}
