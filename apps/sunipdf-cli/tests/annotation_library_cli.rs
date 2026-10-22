use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use sunipdf_core::annotation_library::MAX_LIBRARY_BYTES;
use sunipdf_core::Protection;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/sunipdf-core/tests/fixtures/two-pages.pdf")
}

fn cli(action: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
    command.args(["annotation-library", action]);
    command
}

fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn failure(output: Output) -> Value {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    serde_json::from_slice(&output.stderr).unwrap()
}

fn write_json(path: &Path, value: &Value) {
    std::fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}

fn group(id: &str, name: &str) -> Value {
    json!({
        "id":id,"name":name,
        "provenance":{"author":"Plan reviewer","source":"Locally authored","license":"CC0-1.0"},
        "primitives":[
            {"kind":"shape","shape":"square","rect":[0,0,80,40],
             "style":{"rgb":[200,30,40],"fill_rgb":null,"width_pt":1.5,"opacity":0.8,"dash_pattern":[]}},
            {"kind":"stamp","rect":[90,0,170,40],"stamp":"approved"}
        ]
    })
}

fn library(entries: Vec<Value>) -> Value {
    json!({"format":"sunipdf-annotation-library","version":1,"entries":entries})
}

fn seed(directory: &Path) -> PathBuf {
    let path = directory.join("symbols.json");
    write_json(&path, &library(vec![group("review", "Review group")]));
    path
}

fn place(library: &Path, source: &Path, out: &Path) -> Command {
    let mut command = cli("place");
    command
        .arg(library)
        .args(["--id", "review", "--file"])
        .arg(source)
        .args(["--page", "1", "--origin", "40,60", "--scale", "1", "--out"])
        .arg(out);
    command
}

fn piped(mut command: Command, password: &str) -> Output {
    command.arg("--password-stdin");
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(password.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn annotations(bytes: &[u8], password: Option<&str>, page: u32) -> Vec<lopdf::Dictionary> {
    let pdf = lopdf::Document::load_mem_with_options(
        bytes,
        lopdf::LoadOptions::with_password(password.unwrap_or_default()),
    )
    .unwrap();
    let page = pdf.get_dictionary(pdf.get_pages()[&page]).unwrap();
    let Ok(raw) = page.get(b"Annots") else {
        return Vec::new();
    };
    pdf.dereference(raw)
        .unwrap()
        .1
        .as_array()
        .unwrap()
        .iter()
        .map(|value| pdf.dereference(value).unwrap().1.as_dict().unwrap().clone())
        .collect()
}

#[test]
fn libraries_round_trip_groups_select_export_and_remove_without_overwriting_any_input() {
    let directory = tempfile::tempdir().unwrap();
    let entry = directory.path().join("entry.json");
    let first = directory.path().join("first.json");
    write_json(&entry, &group("review", "Review group"));
    let result = success(
        cli("add")
            .arg("--entry")
            .arg(&entry)
            .arg("--out")
            .arg(&first)
            .output()
            .unwrap(),
    );
    assert_eq!(result["entries"], 1);
    let before = std::fs::read(&first).unwrap();
    let inspected = success(cli("inspect").arg(&first).output().unwrap());
    assert_eq!(
        inspected["entries"][0]["primitives"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(inspected["entries"][0]["provenance"]["license"], "CC0-1.0");

    let second = directory.path().join("second.json");
    write_json(&entry, &group("check", "Check group"));
    assert_eq!(
        success(
            cli("add")
                .arg("--library")
                .arg(&first)
                .arg("--entry")
                .arg(&entry)
                .arg("--out")
                .arg(&second)
                .output()
                .unwrap()
        )["entries"],
        2
    );
    let selected = directory.path().join("selected.json");
    assert_eq!(
        success(
            cli("export")
                .arg(&second)
                .args(["--id", "check", "--out"])
                .arg(&selected)
                .output()
                .unwrap()
        )["entries"],
        1
    );
    let selected_json = success(cli("inspect").arg(&selected).output().unwrap());
    assert_eq!(selected_json["entries"][0]["id"], "check");
    let removed = directory.path().join("removed.json");
    assert_eq!(
        success(
            cli("remove")
                .arg(&second)
                .args(["--id", "review", "--out"])
                .arg(&removed)
                .output()
                .unwrap()
        )["entries"],
        1
    );
    assert_eq!(
        success(cli("inspect").arg(&removed).output().unwrap()),
        selected_json
    );
    let exported = directory.path().join("exported.json");
    success(
        cli("export")
            .arg(&first)
            .arg("--out")
            .arg(&exported)
            .output()
            .unwrap(),
    );
    assert_eq!(
        success(cli("inspect").arg(&exported).output().unwrap()),
        inspected
    );
    assert_eq!(std::fs::read(&first).unwrap(), before);

    for action in ["remove", "export"] {
        let out = directory.path().join(format!("{action}-missing.json"));
        failure(
            cli(action)
                .arg(&second)
                .args(["--id", "missing", "--out"])
                .arg(&out)
                .output()
                .unwrap(),
        );
        assert!(!out.exists());
    }
    let out = directory.path().join("duplicate.json");
    failure(
        cli("export")
            .arg(&second)
            .args(["--id", "review", "--id", "review", "--out"])
            .arg(&out)
            .output()
            .unwrap(),
    );
    assert!(!out.exists());
    for destination in [&first, &second] {
        let original = std::fs::read(destination).unwrap();
        assert_eq!(
            failure(
                cli("export")
                    .arg(&first)
                    .arg("--out")
                    .arg(destination)
                    .arg("--overwrite")
                    .output()
                    .unwrap()
            )["error"],
            "already_exists"
        );
        assert_eq!(std::fs::read(destination).unwrap(), original);
    }
}

#[test]
fn imports_refuse_unknown_versions_fields_scripts_paths_calibration_duplicate_keys_and_excess() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("invalid.json");
    let out = directory.path().join("must-not-exist.json");
    let valid = library(vec![group("review", "Review group")]);
    let mut variants = Vec::new();
    let mut invalid = valid.clone();
    invalid["version"] = json!(2);
    variants.push(invalid);
    let mut invalid = valid.clone();
    invalid["script"] = json!("run");
    variants.push(invalid);
    let mut invalid = valid.clone();
    invalid["entries"][0]["primitives"][0]["path"] = json!("hidden.png");
    variants.push(invalid);
    let mut invalid = valid.clone();
    invalid["entries"][0]["primitives"][0]["calibration"] = json!({"distance":10});
    variants.push(invalid);
    let mut invalid = valid.clone();
    invalid["entries"][0]["primitives"][0]["kind"] = json!("javascript");
    variants.push(invalid);
    let mut invalid = valid.clone();
    invalid["entries"][0]["primitives"] = json!([]);
    variants.push(invalid);
    variants.push(library(vec![
        group("same", "First"),
        group("same", "Second"),
    ]));
    variants.push(library(vec![
        group("first", "Same"),
        group("second", "Same"),
    ]));
    for invalid in variants {
        write_json(&input, &invalid);
        failure(cli("inspect").arg(&input).output().unwrap());
        failure(
            cli("export")
                .arg(&input)
                .arg("--out")
                .arg(&out)
                .output()
                .unwrap(),
        );
        assert!(!out.exists());
    }
    std::fs::write(
        &input,
        b"{\"format\":\"sunipdf-annotation-library\",\"version\":2,\"version\":1,\"entries\":[]}",
    )
    .unwrap();
    failure(cli("inspect").arg(&input).output().unwrap());
    let entry = serde_json::to_string(&group("review", "Review group")).unwrap();
    let duplicate = format!("{{\"id\":\"first\",{}", &entry[1..]);
    std::fs::write(&input, duplicate).unwrap();
    failure(
        cli("add")
            .arg("--entry")
            .arg(&input)
            .arg("--out")
            .arg(&out)
            .output()
            .unwrap(),
    );
    assert!(!out.exists());
    std::fs::write(&input, vec![b' '; MAX_LIBRARY_BYTES + 1]).unwrap();
    failure(cli("inspect").arg(&input).output().unwrap());
    assert!(!out.exists());
}

#[test]
fn placement_adds_the_whole_group_with_fresh_ids_and_refuses_partial_or_existing_outputs() {
    let directory = tempfile::tempdir().unwrap();
    let library = seed(directory.path());
    let source = fixture();
    let original = std::fs::read(&source).unwrap();
    let first = directory.path().join("first.pdf");
    let result = success(place(&library, &source, &first).output().unwrap());
    assert_eq!(result["count"], 2);
    assert_eq!(result["indices"], json!([0, 1]));
    assert_eq!(result["original_unchanged"], true);
    assert_eq!(result["source_content_exported"], false);
    let first_bytes = std::fs::read(&first).unwrap();
    let inserted = annotations(&first_bytes, None, 1);
    assert_eq!(inserted.len(), 2);
    assert!(annotations(&first_bytes, None, 2).is_empty());
    for annotation in &inserted {
        assert!(annotation.has(b"AP"));
    }
    let second = directory.path().join("second.pdf");
    success(place(&library, &first, &second).output().unwrap());
    let twice = annotations(&std::fs::read(&second).unwrap(), None, 1);
    assert_eq!(twice.len(), 4);
    let ids: Vec<_> = twice.iter().map(|a| a.get(b"NM").unwrap()).collect();
    for (i, id) in ids.iter().enumerate() {
        assert!(!ids[..i].contains(id));
    }
    assert_eq!(&twice[..2], inserted.as_slice());
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert_eq!(std::fs::read(&first).unwrap(), first_bytes);

    let bad_library = directory.path().join("out-of-page.json");
    let mut oversized = group("review", "Review group");
    oversized["primitives"][1]["rect"] = json!([900, 0, 980, 40]);
    write_json(&bad_library, &library_value(oversized));
    let refused = directory.path().join("refused.pdf");
    failure(place(&bad_library, &source, &refused).output().unwrap());
    assert!(!refused.exists());
    for destination in [&source, &first] {
        let before = std::fs::read(destination).unwrap();
        assert_eq!(
            failure(
                place(&library, &source, destination)
                    .arg("--overwrite")
                    .output()
                    .unwrap()
            )["error"],
            "already_exists"
        );
        assert_eq!(std::fs::read(destination).unwrap(), before);
    }
    assert_eq!(std::fs::read(&source).unwrap(), original);
}

fn library_value(entry: Value) -> Value {
    library(vec![entry])
}

#[test]
fn encrypted_placement_preserves_permissions_and_never_exports_restricted_pdf_content() {
    let directory = tempfile::tempdir().unwrap();
    let library = seed(directory.path());
    let source_bytes = std::fs::read(fixture()).unwrap();
    let opening = format!("library-opening-{}", std::process::id());
    let owner = format!("library-owner-{}", std::process::id());
    for aes256 in [false, true] {
        for allowed in [false, true] {
            let source = directory.path().join(format!("{aes256}-{allowed}.pdf"));
            let bytes = sunipdf_core::security::protect(
                &source_bytes,
                None,
                &Protection {
                    user_password: opening.clone(),
                    owner_password: owner.clone(),
                    allow_copy: false,
                    allow_modify: false,
                    allow_print: false,
                    allow_annotate: allowed,
                    aes256,
                },
            )
            .unwrap();
            std::fs::write(&source, &bytes).unwrap();
            let out = directory
                .path()
                .join(format!("{aes256}-{allowed}-placed.pdf"));
            let mut command = place(&library, &source, &out);
            command
                .args(["--password-env", "SUNIPDF_LIBRARY_TEST_PASSWORD"])
                .env("SUNIPDF_LIBRARY_TEST_PASSWORD", &opening);
            let output = command.output().unwrap();
            if allowed {
                let text = String::from_utf8(output.stdout.clone()).unwrap();
                let result = success(output);
                assert_eq!(result["encrypted"], true);
                assert_eq!(result["count"], 2);
                for field in [
                    "text",
                    "contents",
                    "primitives",
                    "metadata",
                    "rgba",
                    "provenance",
                ] {
                    assert!(!text.contains(&format!("\"{field}\"")));
                }
                let mut properties = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
                properties.arg("properties").arg(&out);
                let properties = success(piped(properties, &opening));
                for field in ["copy", "modify", "print"] {
                    assert_eq!(properties["permissions"][field], false);
                }
                assert_eq!(properties["permissions"]["annotate"], true);
                assert_eq!(
                    annotations(&std::fs::read(&out).unwrap(), Some(&opening), 1).len(),
                    2
                );
                let next = directory
                    .path()
                    .join(format!("{aes256}-{allowed}-stdin.pdf"));
                assert_eq!(
                    success(piped(place(&library, &source, &next), &opening))["count"],
                    2
                );
            } else {
                assert_eq!(failure(output)["error"], "security_restricted");
                assert!(!out.exists());
                assert_eq!(
                    failure(piped(place(&library, &source, &out), &owner))["error"],
                    "security_restricted"
                );
                assert!(!out.exists());
            }
            assert_eq!(std::fs::read(&source).unwrap(), bytes);
        }
    }
}
