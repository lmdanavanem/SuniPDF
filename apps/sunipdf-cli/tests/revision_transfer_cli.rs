use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}

fn failure(output: Output) -> Value {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    serde_json::from_slice(&output.stderr).unwrap()
}

fn transfer(action: &str, old: &Path, new: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
    command
        .args(["revision-transfer", action])
        .arg(old)
        .arg(new);
    command
}

fn write_json(path: &Path, value: &Value) {
    std::fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

fn seed(path: &Path, with_field: bool) {
    use lopdf::{dictionary, Document, Object, Stream};
    let mut pdf = Document::with_version("1.7");
    let pages = pdf.new_object_id();
    let mut kids = Vec::new();
    let mut fields = Vec::new();
    for number in 0..2 {
        let content = pdf.add_object(Stream::new(
            dictionary! {},
            b"0 0 1 rg 10 10 12 12 re f\n".to_vec(),
        ));
        let page_id = pdf.new_object_id();
        let mut page = dictionary! {
            "Type"=>"Page", "Parent"=>pages,
            "MediaBox"=>vec![0.into(),0.into(),400.into(),300.into()],
            "Resources"=>dictionary! {}, "Contents"=>content,
        };
        if number == 1 && with_field {
            let widget = pdf.add_object(dictionary! {
                "Type"=>"Annot", "Subtype"=>"Widget", "FT"=>"Tx",
                "T"=>Object::string_literal("Existing field"),
                "V"=>Object::string_literal("Keep existing value"),
                "Rect"=>vec![20.into(),20.into(),160.into(),40.into()], "P"=>page_id, "F"=>4,
            });
            page.set("Annots", vec![Object::Reference(widget)]);
            fields.push(Object::Reference(widget));
        }
        pdf.objects.insert(page_id, Object::Dictionary(page));
        kids.push(Object::Reference(page_id));
    }
    pdf.objects.insert(
        pages,
        dictionary! {"Type"=>"Pages","Kids"=>kids,"Count"=>2}.into(),
    );
    let mut root = dictionary! {"Type"=>"Catalog","Pages"=>pages};
    if with_field {
        root.set(
            "AcroForm",
            dictionary! {"Fields"=>fields,"NeedAppearances"=>false},
        );
    }
    let root_id = pdf.add_object(root);
    pdf.trailer.set("Root", root_id);
    pdf.save(path).unwrap();
}

fn sources(directory: &Path) -> (PathBuf, PathBuf) {
    let library = directory.join("native shapes.json");
    write_json(
        &library,
        &json!({
            "format":"sunipdf-annotation-library","version":1,
            "entries":[{"id":"review","name":"Review shapes",
                "provenance":{"author":"Review team","source":"Synthetic fixture","license":"CC0-1.0"},
                "primitives":[
                    {"kind":"shape","shape":"square","rect":[0,0,60,40],
                     "style":{"rgb":[230,30,20],"fill_rgb":[230,30,20],"width_pt":1.5,"opacity":1.0,"dash_pattern":[]}},
                    {"kind":"stamp","rect":[80,0,120,40],"stamp":"approved"}
                ]}]
        }),
    );
    let mut outputs = Vec::new();
    for (label, origin, field) in [
        ("old résumé", "40,60", false),
        ("new révision", "200,150", true),
    ] {
        let base = directory.join(format!("{label} base.pdf"));
        seed(&base, field);
        let out = directory.join(format!("{label}.pdf"));
        let mut command = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
        command
            .args(["annotation-library", "place"])
            .arg(&library)
            .args(["--id", "review", "--file"])
            .arg(&base)
            .args([
                "--page",
                "1",
                "--origin",
                origin,
                "--scale",
                "1",
                "--author",
                "Reviewer Été",
                "--out",
            ])
            .arg(&out);
        success(command.output().unwrap());
        outputs.push(out);
    }
    (outputs.remove(0), outputs.remove(0))
}

fn inspected(old: &Path, new: &Path) -> Value {
    success(
        transfer("inspect", old, new)
            .args(["--pages", "1:1,2:2"])
            .output()
            .unwrap(),
    )
}

fn recipe(inspection: &Value) -> Value {
    let selected = inspection["pages"][0]["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|candidate| candidate["fingerprint"].is_string())
        .unwrap();
    json!({
        "version":1,"old_sha256":inspection["old_sha256"],"new_sha256":inspection["new_sha256"],
        "acknowledge_repeated_transfer":false,
        "pairs":[
            {"old_page":1,"new_page":1,"old_sheet_id":null,"new_sheet_id":null,
             "confirmed":true,"alignment":{"mode":"identity"},
             "selections":[{"index":selected["index"],"fingerprint":selected["fingerprint"]}]},
            {"old_page":2,"new_page":2,"old_sheet_id":null,"new_sheet_id":null,
             "confirmed":true,"alignment":{"mode":"identity"},"selections":[]}
        ]
    })
}

fn apply(old: &Path, new: &Path, recipe: &Path, output: &Path) -> Command {
    let mut command = transfer("apply", old, new);
    command.arg("--recipe").arg(recipe).arg("--out").arg(output);
    command
}

fn annotation_names(path: &Path, page: u32, password: Option<&str>) -> Vec<Vec<u8>> {
    let pdf = lopdf::Document::load_with_options(
        path,
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
        .map(|object| {
            pdf.dereference(object)
                .unwrap()
                .1
                .as_dict()
                .unwrap()
                .get(b"NM")
                .unwrap()
                .as_str()
                .unwrap()
                .to_vec()
        })
        .collect()
}

fn piped(mut command: Command, password: &str) -> Output {
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

fn assert_no_secret(output: &Output, secrets: &[&str]) {
    for secret in secrets {
        assert!(!String::from_utf8_lossy(&output.stdout).contains(secret));
        assert!(!String::from_utf8_lossy(&output.stderr).contains(secret));
    }
}

fn decoded_string(object: &lopdf::Object) -> String {
    let bytes = object.as_str().unwrap();
    if bytes.starts_with(&[0xfe, 0xff]) {
        let units = bytes[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>();
        String::from_utf16(&units).unwrap()
    } else {
        String::from_utf8(bytes.to_vec()).unwrap()
    }
}

#[test]
fn inspection_real_preview_and_new_copy_preserve_sources_fields_and_existing_annotations() {
    let directory = tempfile::tempdir().unwrap();
    let (old, new) = sources(directory.path());
    let old_bytes = std::fs::read(&old).unwrap();
    let new_bytes = std::fs::read(&new).unwrap();
    let existing_names = annotation_names(&new, 1, None);
    let inspection = inspected(&old, &new);
    assert_eq!(inspection["complete"], true);
    assert_eq!(inspection["old_page_count"], 2);
    assert_eq!(inspection["new_page_count"], 2);
    assert_eq!(inspection["page_index_base"], 1);
    assert_eq!(inspection["annotation_index_base"], 0);
    assert!(inspection["pages"][0]["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["refusal"].is_string()));
    assert!(inspection["pages"][1]["candidates"]
        .as_array()
        .unwrap()
        .is_empty());
    let recipe_path = directory.path().join("confirmed recipe.json");
    write_json(&recipe_path, &recipe(&inspection));
    let recipe_bytes = std::fs::read(&recipe_path).unwrap();
    let report_only = success(
        transfer("preview", &old, &new)
            .arg("--recipe")
            .arg(&recipe_path)
            .output()
            .unwrap(),
    );
    assert_eq!(report_only["published"], false);
    assert!(report_only["output_path"].is_null());
    assert!(report_only["preview_image"].is_null());
    assert!(!report_only["exclusions"].as_array().unwrap().is_empty());
    let preview = directory.path().join("candidate.png");
    let preview_command = || {
        let mut command = transfer("preview", &old, &new);
        command
            .arg("--recipe")
            .arg(&recipe_path)
            .arg("--preview-out")
            .arg(&preview)
            .args(["--preview-page", "1", "--preview-long-side", "192"]);
        command
    };
    let receipt = success(preview_command().output().unwrap());
    let preview_bytes = std::fs::read(&preview).unwrap();
    assert_eq!(&preview_bytes[..8], b"\x89PNG\r\n\x1a\n");
    let pixels = image::load_from_memory(&preview_bytes)
        .unwrap()
        .into_rgba8();
    assert!(pixels.width().max(pixels.height()) <= 192);
    assert_eq!(receipt["preview_image"]["width"], pixels.width());
    assert_eq!(receipt["preview_image"]["height"], pixels.height());
    // The transferred red square is absent from this location in the unmodified target.
    let pixel = pixels
        .get_pixel(pixels.width() * 70 / 400, pixels.height() * 220 / 300)
        .0;
    assert!(
        pixel[0] > 180 && pixel[1] < 100 && pixel[2] < 100,
        "{pixel:?}"
    );
    let repeated_preview = preview_command().arg("--overwrite").output().unwrap();
    assert_eq!(repeated_preview.status.code(), Some(6));
    failure(repeated_preview);
    assert_eq!(std::fs::read(&preview).unwrap(), preview_bytes);
    let output = directory.path().join("transferred.pdf");
    let applied = success(apply(&old, &new, &recipe_path, &output).output().unwrap());
    assert_eq!(applied["published"], true);
    assert_eq!(applied["transferred"].as_array().unwrap().len(), 1);
    assert_eq!(applied["output_sha256"].as_str().unwrap().len(), 64);
    assert_eq!(applied["old_sha256"], inspection["old_sha256"]);
    assert_eq!(applied["new_sha256"], inspection["new_sha256"]);
    assert_ne!(
        applied["transferred"][0]["source_nm"],
        applied["transferred"][0]["target_nm"]
    );
    assert_eq!(applied["transferred"][0]["author"], "Reviewer Été");
    let names = annotation_names(&output, 1, None);
    assert_eq!(names.len(), existing_names.len() + 1);
    assert_eq!(&names[..existing_names.len()], existing_names.as_slice());
    let pdf = lopdf::Document::load(&output).unwrap();
    assert_eq!(pdf.get_pages().len(), 2);
    assert!(pdf
        .objects
        .values()
        .filter_map(|object| object.as_dict().ok())
        .any(|dict| {
            dict.get(b"T")
                .ok()
                .is_some_and(|value| decoded_string(value) == "Existing field")
                && dict
                    .get(b"V")
                    .ok()
                    .is_some_and(|value| decoded_string(value) == "Keep existing value")
        }));
    let output_bytes = std::fs::read(&output).unwrap();
    let existing = apply(&old, &new, &recipe_path, &output)
        .arg("--overwrite")
        .output()
        .unwrap();
    assert_eq!(existing.status.code(), Some(6));
    failure(existing);
    assert_eq!(std::fs::read(&output).unwrap(), output_bytes);
    failure(
        apply(&old, &new, &recipe_path, &new)
            .arg("--overwrite")
            .output()
            .unwrap(),
    );
    let report_file = directory.path().join("preview receipt.json");
    write_json(&report_file, &receipt);
    let forbidden = directory.path().join("receipt-is-not-recipe.pdf");
    failure(
        apply(&old, &new, &report_file, &forbidden)
            .output()
            .unwrap(),
    );
    assert!(!forbidden.exists());
    assert_eq!(std::fs::read(&old).unwrap(), old_bytes);
    assert_eq!(std::fs::read(&new).unwrap(), new_bytes);
    assert_eq!(std::fs::read(&recipe_path).unwrap(), recipe_bytes);
    let repeated_recipe = directory.path().join("repeated transfer.json");
    let mut repeated = recipe(&inspected(&old, &output));
    write_json(&repeated_recipe, &repeated);
    let repeated_output = directory.path().join("explicitly repeated.pdf");
    failure(
        apply(&old, &output, &repeated_recipe, &repeated_output)
            .output()
            .unwrap(),
    );
    assert!(!repeated_output.exists());
    repeated["acknowledge_repeated_transfer"] = json!(true);
    write_json(&repeated_recipe, &repeated);
    success(
        apply(&old, &output, &repeated_recipe, &repeated_output)
            .output()
            .unwrap(),
    );
    assert_eq!(
        annotation_names(&repeated_output, 1, None).len(),
        names.len() + 1
    );
    assert_eq!(std::fs::read(&output).unwrap(), output_bytes);
}

#[test]
fn recipe_and_source_guards_refuse_stale_unconfirmed_unsupported_and_oversized_inputs() {
    let directory = tempfile::tempdir().unwrap();
    let (old, new) = sources(directory.path());
    let inspection = inspected(&old, &new);
    let good = recipe(&inspection);
    let recipe_path = directory.path().join("recipe.json");
    let output = directory.path().join("must-not-exist.pdf");
    let mut bad_recipes = Vec::new();
    let mut value = good.clone();
    value["old_sha256"] = json!("0".repeat(64));
    bad_recipes.push(value);
    let mut value = good.clone();
    value["pairs"][0]["confirmed"] = json!(false);
    bad_recipes.push(value);
    let mut value = good.clone();
    value["pairs"][0]["selections"][0]["fingerprint"] = json!("0".repeat(64));
    bad_recipes.push(value);
    let mut value = good.clone();
    value["pairs"][0]["selections"][0]["index"] = json!(1);
    bad_recipes.push(value);
    let mut value = good.clone();
    value["pairs"][1]["new_page"] = json!(1);
    bad_recipes.push(value);
    let mut value = good.clone();
    value["unexpected"] = json!(true);
    bad_recipes.push(value);
    let mut value = good.clone();
    value["pairs"][0]["alignment"] =
        json!({"mode":"anchors","old_points":[[0,0],[100,0]],"new_points":[[0,0],[100,100]]});
    bad_recipes.push(value);
    for value in bad_recipes {
        write_json(&recipe_path, &value);
        failure(apply(&old, &new, &recipe_path, &output).output().unwrap());
        assert!(!output.exists());
    }
    write_json(&recipe_path, &good);
    for path in [&old, &new] {
        let before = std::fs::read(path).unwrap();
        let mut changed = before.clone();
        changed.extend_from_slice(b"\n% changed after inspection\n");
        std::fs::write(path, &changed).unwrap();
        failure(apply(&old, &new, &recipe_path, &output).output().unwrap());
        assert!(!output.exists());
        assert_eq!(std::fs::read(path).unwrap(), changed);
        std::fs::write(path, before).unwrap();
    }
    std::fs::write(&recipe_path, vec![b' '; 256 * 1024 + 1]).unwrap();
    failure(apply(&old, &new, &recipe_path, &output).output().unwrap());
    failure(
        apply(&old, &new, directory.path(), &output)
            .output()
            .unwrap(),
    );
    assert!(!output.exists());
}

#[test]
fn empty_inspection_keeps_json_with_exit_two_and_page_arguments_remain_explicit() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("empty annotations.pdf");
    seed(&source, false);
    let before = std::fs::read(&source).unwrap();
    let output = transfer("inspect", &source, &source)
        .args(["--pages", "1:1"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["complete"], false);
    assert!(value["pages"][0]["candidates"]
        .as_array()
        .unwrap()
        .is_empty());
    for pairs in ["0:1", "1:1,2:1", "1:1,1:2", "all", "1:2:3"] {
        let output = transfer("inspect", &source, &source)
            .args(["--pages", pairs])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
    failure(
        transfer("inspect", &source, &source)
            .args(["--pages", "3:1"])
            .output()
            .unwrap(),
    );
    let missing = directory.path().join("missing.pdf");
    failure(
        transfer("inspect", &missing, &source)
            .args(["--pages", "1:1"])
            .output()
            .unwrap(),
    );
    assert_eq!(std::fs::read(&source).unwrap(), before);
}

fn protect(source: &Path, out: &Path, opening: &str, owner: &str, denied: Option<&str>) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
    command
        .arg("protect")
        .arg(source)
        .args([
            "--user-password-env",
            "SUNIPDF_TRANSFER_OPENING",
            "--owner-password-env",
            "SUNIPDF_TRANSFER_OWNER",
            "--out",
        ])
        .arg(out)
        .env("SUNIPDF_TRANSFER_OPENING", opening)
        .env("SUNIPDF_TRANSFER_OWNER", owner);
    if let Some(flag) = denied {
        command.arg(flag);
    }
    let output = command.output().unwrap();
    assert_no_secret(&output, &[opening, owner]);
    success(output);
}

#[test]
fn two_distinct_secure_passwords_work_and_permissions_stdin_conflicts_and_missing_secrets_refuse() {
    let directory = tempfile::tempdir().unwrap();
    let (old_plain, new_plain) = sources(directory.path());
    let old = directory.path().join("old protected.pdf");
    let new = directory.path().join("new protected.pdf");
    let old_secret = format!("source-été-{}", std::process::id());
    let new_secret = format!("target-été-{}", std::process::id());
    let owner = format!("owner-{}", std::process::id());
    protect(&old_plain, &old, &old_secret, &owner, None);
    protect(&new_plain, &new, &new_secret, &owner, None);
    let before_old = std::fs::read(&old).unwrap();
    let before_new = std::fs::read(&new).unwrap();
    let secured = |mut command: Command| {
        command
            .args([
                "--password-env",
                "SUNIPDF_TRANSFER_OLD",
                "--new-password-env",
                "SUNIPDF_TRANSFER_NEW",
            ])
            .env("SUNIPDF_TRANSFER_OLD", &old_secret)
            .env("SUNIPDF_TRANSFER_NEW", &new_secret);
        command
    };
    let inspected_output = secured({
        let mut command = transfer("inspect", &old, &new);
        command.args(["--pages", "1:1,2:2"]);
        command
    })
    .output()
    .unwrap();
    assert_no_secret(&inspected_output, &[&old_secret, &new_secret, &owner]);
    let inspection = success(inspected_output);
    let recipe_path = directory.path().join("protected recipe.json");
    write_json(&recipe_path, &recipe(&inspection));
    let output = directory.path().join("protected transferred.pdf");
    let mut command = apply(&old, &new, &recipe_path, &output);
    command
        .args([
            "--password-env",
            "SUNIPDF_TRANSFER_OLD",
            "--new-password-stdin",
        ])
        .env("SUNIPDF_TRANSFER_OLD", &old_secret);
    assert!(!command
        .get_args()
        .any(|arg| arg == old_secret.as_str() || arg == new_secret.as_str()));
    let applied = piped(command, &format!("{new_secret}\r\n"));
    assert_no_secret(&applied, &[&old_secret, &new_secret, &owner]);
    success(applied);
    let mut uncredentialed = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"));
    let uncredentialed = uncredentialed.arg("info").arg(&output).output().unwrap();
    assert_eq!(uncredentialed.status.code(), Some(3));
    failure(uncredentialed);
    assert_eq!(annotation_names(&output, 1, Some(&new_secret)).len(), 3);
    let denied_output = directory.path().join("denied.pdf");
    let mut conflict = apply(&old, &new, &recipe_path, &denied_output);
    conflict.args(["--password-stdin", "--new-password-stdin"]);
    let conflict = piped(conflict, &new_secret);
    assert_no_secret(&conflict, &[&new_secret]);
    failure(conflict);
    let wrong = secured(apply(&old, &new, &recipe_path, &denied_output))
        .env("SUNIPDF_TRANSFER_NEW", &old_secret)
        .output()
        .unwrap();
    assert_no_secret(&wrong, &[&old_secret, &new_secret]);
    failure(wrong);
    let missing = secured(apply(&old, &new, &recipe_path, &denied_output))
        .env_remove("SUNIPDF_TRANSFER_NEW")
        .output()
        .unwrap();
    assert_no_secret(&missing, &[&old_secret, &new_secret]);
    failure(missing);
    let blocked_old = directory.path().join("old no copy.pdf");
    protect(
        &old_plain,
        &blocked_old,
        &old_secret,
        &owner,
        Some("--no-copy"),
    );
    let mut command = transfer("inspect", &blocked_old, &new);
    command.args(["--pages", "1:1"]);
    failure(secured(command).output().unwrap());
    let blocked_new = directory.path().join("new no annotations.pdf");
    protect(
        &new_plain,
        &blocked_new,
        &new_secret,
        &owner,
        Some("--no-annotate"),
    );
    let mut command = transfer("inspect", &old, &blocked_new);
    command.args(["--pages", "1:1"]);
    failure(secured(command).output().unwrap());
    assert!(!denied_output.exists());
    assert_eq!(std::fs::read(&old).unwrap(), before_old);
    assert_eq!(std::fs::read(&new).unwrap(), before_new);
}
