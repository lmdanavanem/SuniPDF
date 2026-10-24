use lopdf::{dictionary, Document, Object, Stream};
use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

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

fn refused(output: Output, code: i32) -> Value {
    assert_eq!(output.status.code(), Some(code));
    assert!(output.stdout.is_empty());
    serde_json::from_slice(&output.stderr).unwrap()
}

fn fixture_pdf(clean: bool) -> Vec<u8> {
    let mut pdf = Document::with_version("1.7");
    let pages = pdf.new_object_id();
    let content = pdf.add_object(Stream::new(dictionary! {}, b"0 g 5 5 2 2 re f".to_vec()));
    let bounds = [
        Some([30, 30, 40, 40]),
        Some([100, 100, 110, 110]),
        Some([15, 25, 25, 35]),
        Some([220, 30, 230, 40]),
        Some([290, 150, 300, 160]),
        None,
    ];
    let mut children = Vec::new();
    for page_number in 1..=2 {
        let mut annotations = Vec::new();
        for (index, bounds) in bounds.iter().enumerate() {
            if clean && [2, 3, 5].contains(&index) {
                continue;
            }
            let mut annotation = dictionary! {
                "Type"=>"Annot", "Subtype"=>"Stamp", "Name"=>"SuniPDFCount",
                "NM"=>Object::string_literal(format!("SuniPDF-count-{page_number}-{index}")),
                "Contents"=>Object::string_literal(format!("Fixture marker {index}"))
            };
            if let Some(bounds) = bounds {
                annotation.set(
                    "Rect",
                    bounds
                        .iter()
                        .copied()
                        .map(Object::Integer)
                        .collect::<Vec<_>>(),
                );
                annotation.set(
                    "SuniPDFCount",
                    Object::string_literal(br#"{"version":1,"group":"Equipment"}"#.to_vec()),
                );
            }
            annotations.push(Object::Reference(pdf.add_object(annotation)));
        }
        children.push(Object::Reference(pdf.add_object(
            dictionary! {"Type"=>"Page", "Parent"=>pages,
            "MediaBox"=>vec![0.into(),0.into(),400.into(),300.into()], "Resources"=>dictionary!{},
            "Contents"=>content, "Annots"=>annotations},
        )));
    }
    pdf.objects.insert(
        pages,
        dictionary! {"Type"=>"Pages", "Count"=>2, "Kids"=>children}.into(),
    );
    let root = pdf.add_object(dictionary! {"Type"=>"Catalog", "Pages"=>pages});
    pdf.trailer.set("Root", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

fn zones() -> Value {
    json!({"version":1,"zones":[
        {"id":"floor","name":"Ground floor","parent_id":null,"bounds":[10,10,180,180]},
        {"id":"room","name":"Room A","parent_id":"floor","bounds":[20,20,60,60]},
        {"id":"overlap_a","name":"Overlap A","parent_id":null,"bounds":[200,10,260,80]},
        {"id":"overlap_b","name":"Overlap B","parent_id":null,"bounds":[210,20,270,90]}
    ]})
}

struct Fixture {
    directory: tempfile::TempDir,
    source: PathBuf,
    zones: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        Self::with_geometry(false)
    }

    fn with_geometry(clean: bool) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.pdf");
        let zones_path = directory.path().join("zones.json");
        std::fs::write(&source, fixture_pdf(clean)).unwrap();
        std::fs::write(&zones_path, serde_json::to_vec(&zones()).unwrap()).unwrap();
        Self {
            directory,
            source,
            zones: zones_path,
        }
    }

    fn output(&self, name: &str) -> PathBuf {
        self.directory.path().join(name)
    }

    fn command(&self, operation: &str, source: &Path) -> Command {
        let mut command = cli();
        command.args(["plan-zones", operation]).arg(source);
        if operation != "inspect" {
            command.arg("--zones").arg(&self.zones);
        }
        command
    }
}

#[test]
fn replacement_preview_round_trips_and_shared_filters_do_not_double_count() {
    let fixture = Fixture::new();
    let original = std::fs::read(&fixture.source).unwrap();
    let inspected = ok(
        fixture
            .command("inspect", &fixture.source)
            .output()
            .unwrap(),
        0,
    );
    assert_eq!(inspected["zones"]["zones"], json!([]));
    let png = fixture.output("candidate.png");
    let preview = ok(
        fixture
            .command("preview", &fixture.source)
            .arg("--preview-out")
            .arg(&png)
            .args(["--preview-long-side", "200"])
            .output()
            .unwrap(),
        0,
    );
    assert_eq!(preview["published"], false);
    assert_eq!(preview["affected_rows"], 4);
    assert_eq!(image::open(&png).unwrap().width(), 200);
    let assignments = preview["preview"]["assignments"].as_array().unwrap();
    assert_eq!(
        assignments
            .iter()
            .map(|row| row["after"]["status"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "assigned",
            "assigned",
            "boundary",
            "overlap",
            "unassigned",
            "geometry_unknown"
        ]
    );
    assert_eq!(assignments[0]["after"]["zone_id"], "room");
    assert_eq!(assignments[0]["after"]["path"].as_array().unwrap().len(), 2);
    let output = fixture.output("zoned.pdf");
    ok(
        fixture
            .command("apply", &fixture.source)
            .arg("--expected-source-sha256")
            .arg(inspected["source_sha256"].as_str().unwrap())
            .arg("--out")
            .arg(&output)
            .output()
            .unwrap(),
        0,
    );
    assert_eq!(std::fs::read(&fixture.source).unwrap(), original);
    let reopened = ok(fixture.command("inspect", &output).output().unwrap(), 0);
    assert_eq!(reopened["zones"], preview["preview"]["after"]);
    let second = ok(
        fixture
            .command("inspect", &output)
            .args(["--page", "2"])
            .output()
            .unwrap(),
        0,
    );
    assert_eq!(second["zones"]["zones"], json!([]));
    for (extra, expected) in [
        (vec!["--zone-status", "boundary"], 1),
        (vec!["--zone-status", "overlap"], 1),
    ] {
        let register = ok(
            cli()
                .arg("annotation-register")
                .arg(&output)
                .args(&extra)
                .output()
                .unwrap(),
            0,
        );
        let quantities = ok(
            cli()
                .arg("quantity-table")
                .arg(&output)
                .args(&extra)
                .output()
                .unwrap(),
            0,
        );
        assert_eq!(register["matched_annotations"], expected);
        assert_eq!(quantities["summary"]["matched_rows"], expected);
        let register_ids: Vec<_> = register["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["annotation_id"].clone())
            .collect();
        let quantity_ids: Vec<_> = quantities["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["register"]["annotation_id"].clone())
            .collect();
        assert_eq!(
            register_ids, quantity_ids,
            "Both reports must select identical rows"
        );
        assert_eq!(quantities["summary"]["quantity_rows"], expected);
        assert_eq!(quantities["zone_coverage"], register["zone_coverage"]);
        assert_eq!(quantities["zone_coverage"]["complete"], false);
    }
    for command in ["annotation-register", "quantity-table"] {
        let destination = fixture.output(&format!("{command}-refused.csv"));
        refused(
            cli()
                .arg(command)
                .arg(&output)
                .args(["--zone", "1:room", "--format", "csv", "--out"])
                .arg(&destination)
                .output()
                .unwrap(),
            1,
        );
        assert!(
            !destination.exists(),
            "Ambiguous locations must not become an empty room report"
        );
        let unknown = ok(
            cli()
                .arg(command)
                .arg(&output)
                .args(["--zone-status", "geometry_unknown"])
                .output()
                .unwrap(),
            0,
        );
        assert_eq!(unknown["rows"].as_array().unwrap().len(), 2);
    }
}

#[test]
fn known_zone_filters_share_selection_and_reject_unknown_zones_instead_of_zero() {
    let fixture = Fixture::with_geometry(true);
    let output = fixture.output("zoned.pdf");
    ok(
        fixture
            .command("apply", &fixture.source)
            .arg("--out")
            .arg(&output)
            .output()
            .unwrap(),
        0,
    );
    for (extra, expected) in [
        (vec!["--zone", "1:floor"], 1),
        (vec!["--zone", "1:floor", "--zone-descendants"], 2),
        (vec!["--zone", "1:room"], 1),
    ] {
        let register = ok(
            cli()
                .arg("annotation-register")
                .arg(&output)
                .args(&extra)
                .output()
                .unwrap(),
            0,
        );
        let quantities = ok(
            cli()
                .arg("quantity-table")
                .arg(&output)
                .args(&extra)
                .output()
                .unwrap(),
            0,
        );
        assert_eq!(register["matched_annotations"], expected);
        assert_eq!(quantities["summary"]["matched_rows"], expected);
        assert_eq!(quantities["summary"]["quantity_rows"], expected);
        let register_ids: Vec<_> = register["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["annotation_id"].clone())
            .collect();
        let quantity_ids: Vec<_> = quantities["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["register"]["annotation_id"].clone())
            .collect();
        assert_eq!(register_ids, quantity_ids);
        assert_eq!(quantities["zone_coverage"], register["zone_coverage"]);
        assert_eq!(quantities["complete"], true);
    }
    for command in ["annotation-register", "quantity-table"] {
        for selection in ["1:missing", "2:room"] {
            refused(
                cli()
                    .arg(command)
                    .arg(&output)
                    .args(["--zone", selection])
                    .output()
                    .unwrap(),
                1,
            );
        }
        let destination = fixture.output(&format!("{command}.csv"));
        let exported = ok(
            cli()
                .arg(command)
                .arg(&output)
                .args(["--zone", "1:room", "--format", "csv", "--out"])
                .arg(&destination)
                .output()
                .unwrap(),
            0,
        );
        assert_eq!(exported["complete"], true);
        assert_eq!(exported["zone_coverage"]["complete"], true);
        let csv = std::fs::read_to_string(destination).unwrap();
        assert!(csv.contains("zone_status"));
        assert!(csv.contains("SuniPDF-count-1-0"));
        assert!(!csv.contains("SuniPDF-count-1-1"));
    }
}

#[test]
fn each_unresolved_location_refuses_room_totals_but_remains_auditable() {
    for (index, status) in [(2, "boundary"), (3, "overlap"), (5, "geometry_unknown")] {
        let fixture = Fixture::new();
        let mut pdf = Document::load(&fixture.source).unwrap();
        for page in pdf.get_pages().values() {
            let annotations = pdf
                .get_dictionary(*page)
                .unwrap()
                .get(b"Annots")
                .unwrap()
                .as_array()
                .unwrap();
            let selected = annotations[index].clone();
            pdf.get_dictionary_mut(*page)
                .unwrap()
                .set("Annots", vec![selected]);
        }
        pdf.save(&fixture.source).unwrap();
        let original = std::fs::read(&fixture.source).unwrap();
        let output = fixture.output("zoned.pdf");
        ok(
            fixture
                .command("apply", &fixture.source)
                .arg("--out")
                .arg(&output)
                .output()
                .unwrap(),
            0,
        );
        for command in ["annotation-register", "quantity-table"] {
            refused(
                cli()
                    .arg(command)
                    .arg(&output)
                    .args(["--zone", "1:room"])
                    .output()
                    .unwrap(),
                1,
            );
            let audit = ok(
                cli()
                    .arg(command)
                    .arg(&output)
                    .args(["--page", "1", "--zone-status", status])
                    .output()
                    .unwrap(),
                0,
            );
            assert_eq!(audit["rows"].as_array().unwrap().len(), 1);
            assert_eq!(audit["zone_coverage"]["counts"][status], 1);
            assert_eq!(audit["zone_coverage"]["complete"], false);
        }
        assert_eq!(std::fs::read(&fixture.source).unwrap(), original);
    }
}

#[test]
fn stale_content_requires_explicit_reset_and_source_pin_catches_later_file_changes() {
    let fixture = Fixture::new();
    let output = fixture.output("zoned.pdf");
    ok(
        fixture
            .command("apply", &fixture.source)
            .arg("--out")
            .arg(&output)
            .output()
            .unwrap(),
        0,
    );
    let mut pdf = Document::load(&output).unwrap();
    let page_id = pdf.get_pages()[&1];
    let changed = pdf.add_object(Stream::new(
        dictionary! {},
        b"0 g 100 100 20 20 re f".to_vec(),
    ));
    pdf.get_dictionary_mut(page_id)
        .unwrap()
        .set("Contents", changed);
    let stale = fixture.output("stale.pdf");
    pdf.save(&stale).unwrap();
    let before = std::fs::read(&stale).unwrap();
    let inspection = ok(fixture.command("inspect", &stale).output().unwrap(), 2);
    assert_eq!(inspection["zones"]["validity"], "stale");
    assert!(inspection["assignments"]
        .as_array()
        .unwrap()
        .iter()
        .all(|row| row["assignment"]["status"] == "stale"));
    for command in ["annotation-register", "quantity-table"] {
        refused(
            cli()
                .arg(command)
                .arg(&stale)
                .args(["--zone", "1:room"])
                .output()
                .unwrap(),
            1,
        );
        let audit = ok(
            cli()
                .arg(command)
                .arg(&stale)
                .args(["--zone-status", "stale"])
                .output()
                .unwrap(),
            0,
        );
        assert_eq!(audit["rows"].as_array().unwrap().len(), 6);
    }
    let refused_path = fixture.output("refused.pdf");
    refused(
        fixture
            .command("apply", &stale)
            .arg("--out")
            .arg(&refused_path)
            .output()
            .unwrap(),
        1,
    );
    assert!(!refused_path.exists());
    let reset = fixture.output("reset.pdf");
    ok(
        fixture
            .command("apply", &stale)
            .arg("--reset-stale")
            .arg("--out")
            .arg(&reset)
            .output()
            .unwrap(),
        0,
    );
    assert_eq!(
        ok(fixture.command("inspect", &reset).output().unwrap(), 0)["zones"]["validity"],
        "current"
    );
    assert_eq!(std::fs::read(&stale).unwrap(), before);
    let pinned = ok(
        fixture
            .command("preview", &fixture.source)
            .output()
            .unwrap(),
        0,
    )["source_sha256"]
        .as_str()
        .unwrap()
        .to_owned();
    std::fs::OpenOptions::new()
        .append(true)
        .open(&fixture.source)
        .unwrap()
        .write_all(b"\n% another saved revision\n")
        .unwrap();
    refused(
        fixture
            .command("apply", &fixture.source)
            .arg("--expected-source-sha256")
            .arg(pinned)
            .arg("--out")
            .arg(&refused_path)
            .output()
            .unwrap(),
        1,
    );
    assert!(!refused_path.exists());
}

#[test]
fn malformed_bounded_inputs_and_all_output_collisions_are_rejected() {
    let fixture = Fixture::new();
    let before = std::fs::read(&fixture.source).unwrap();
    let output = fixture.output("must-not-exist.pdf");
    for invalid in [
        json!({"version":2,"zones":[]}),
        json!({"version":1,"zones":[],"unknown":true}),
        json!({"version":1,"zones":[{"id":"a","name":"A","parent_id":"missing","bounds":[1,1,20,20]}]}),
        json!({"version":1,"zones":[{"id":"a","name":"A","parent_id":null,"bounds":[0,0,500,500]}]}),
        json!({"version":1,"zones":[{"id":"a","name":"A","parent_id":"b","bounds":[1,1,20,20]},{"id":"b","name":"B","parent_id":"a","bounds":[1,1,20,20]}]}),
    ] {
        std::fs::write(&fixture.zones, serde_json::to_vec(&invalid).unwrap()).unwrap();
        refused(
            fixture
                .command("apply", &fixture.source)
                .arg("--out")
                .arg(&output)
                .output()
                .unwrap(),
            1,
        );
        assert!(!output.exists());
    }
    std::fs::write(&fixture.zones, vec![b' '; 128 * 1024 + 1]).unwrap();
    refused(
        fixture
            .command("preview", &fixture.source)
            .output()
            .unwrap(),
        1,
    );
    std::fs::write(&fixture.zones, serde_json::to_vec(&zones()).unwrap()).unwrap();
    for destination in [&fixture.source, &output] {
        if !destination.exists() {
            std::fs::write(destination, b"preserve existing output").unwrap();
        }
        let existing = std::fs::read(destination).unwrap();
        refused(
            fixture
                .command("apply", &fixture.source)
                .arg("--out")
                .arg(destination)
                .arg("--overwrite")
                .output()
                .unwrap(),
            6,
        );
        assert_eq!(std::fs::read(destination).unwrap(), existing);
    }
    refused(
        fixture
            .command("preview", &fixture.source)
            .arg("--preview-out")
            .arg(&output)
            .arg("--overwrite")
            .output()
            .unwrap(),
        6,
    );
    for flags in [
        vec!["--page", "0"],
        vec!["--expected-source-sha256", "invalid"],
    ] {
        assert_eq!(
            fixture
                .command("inspect", &fixture.source)
                .args(flags)
                .output()
                .unwrap()
                .status
                .code(),
            Some(2)
        );
    }
    for flags in [
        vec!["--zone", "0:room"],
        vec!["--zone", "1:bad:id"],
        vec!["--zone-descendants"],
        vec!["--zone-status", "unknown"],
    ] {
        assert_eq!(
            cli()
                .arg("quantity-table")
                .arg(&fixture.source)
                .args(flags)
                .output()
                .unwrap()
                .status
                .code(),
            Some(2)
        );
    }
    assert_eq!(std::fs::read(&fixture.source).unwrap(), before);
}

#[test]
fn protected_sources_use_ephemeral_env_or_stdin_and_remain_encrypted() {
    let fixture = Fixture::new();
    let protected = fixture.output("protected.pdf");
    let opening = format!("fixture-open-{}", std::process::id());
    let owner = format!("fixture-owner-{}", std::process::id());
    ok(
        cli()
            .arg("protect")
            .arg(&fixture.source)
            .args([
                "--user-password-env",
                "ZONE_OPEN",
                "--owner-password-env",
                "ZONE_OWNER",
                "--out",
            ])
            .arg(&protected)
            .env("ZONE_OPEN", &opening)
            .env("ZONE_OWNER", &owner)
            .output()
            .unwrap(),
        0,
    );
    let before = std::fs::read(&protected).unwrap();
    refused(fixture.command("inspect", &protected).output().unwrap(), 3);
    let output = fixture.output("protected-zones.pdf");
    let response = fixture
        .command("apply", &protected)
        .args(["--password-env", "ZONE_OWNER"])
        .env("ZONE_OWNER", &owner)
        .arg("--out")
        .arg(&output)
        .output()
        .unwrap();
    for secret in [&opening, &owner] {
        assert!(!String::from_utf8_lossy(&response.stdout).contains(secret));
        assert!(!String::from_utf8_lossy(&response.stderr).contains(secret));
    }
    assert_eq!(ok(response, 0)["encrypted"], true);
    refused(fixture.command("inspect", &output).output().unwrap(), 3);
    let mut process = fixture
        .command("inspect", &output)
        .arg("--password-stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    process
        .stdin
        .take()
        .unwrap()
        .write_all(format!("{owner}\n").as_bytes())
        .unwrap();
    let response = process.wait_with_output().unwrap();
    assert!(!String::from_utf8_lossy(&response.stdout).contains(&owner));
    assert!(!String::from_utf8_lossy(&response.stderr).contains(&owner));
    assert_eq!(
        ok(response, 0)["zones"]["zones"].as_array().unwrap().len(),
        4
    );
    assert_eq!(std::fs::read(&protected).unwrap(), before);
}
