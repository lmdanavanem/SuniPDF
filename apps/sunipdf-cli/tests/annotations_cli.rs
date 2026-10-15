//! `annotations`, `annotate`, `reply`, `review-state` et `annotation-delete` de bout en bout
//! (binaire lancé, fichiers rouverts) : exige `pdfium.dll` (scripts/fetch-pdfium.ps1).

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

fn stdout_json(o: &Output) -> serde_json::Value {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    serde_json::from_slice(&o.stdout)
        .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&o.stdout)))
}

fn stderr_json(o: &Output) -> serde_json::Value {
    serde_json::from_slice(&o.stderr)
        .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&o.stderr)))
}

/// Annotations de la page 1 d'un fichier.
fn listed(path: &Path, password: Option<&str>) -> Vec<serde_json::Value> {
    let mut c = cli();
    c.arg("annotations").arg(path).args(["--page", "1"]);
    if let Some(p) = password {
        c.args(["--password", p]);
    }
    let v = stdout_json(&c.output().unwrap());
    v["pages"][0]["annotations"].as_array().unwrap().clone()
}

#[test]
fn annotate_list_reply_status_and_delete_round_trip() {
    let dir = std::env::temp_dir().join(format!("sunipdf-cli-annot-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let base = fixture("two-pages.pdf");

    // Rectangle, puis flèche, trait d'encre, texte libre et lien, chacun relu par `annotations`.
    let step = |input: &Path, name: &str, args: &[&str]| -> PathBuf {
        let out = dir.join(name);
        let o = cli()
            .arg("annotate")
            .arg(input)
            .args(args)
            .arg("--out")
            .arg(&out)
            .output()
            .unwrap();
        stdout_json(&o);
        out
    };
    let a = step(
        &base,
        "a.pdf",
        &[
            "--kind",
            "square",
            "--rect",
            "100,100,200,160",
            "--color",
            "0000FF",
            "--fill",
            "FFFF00",
            "--author",
            "Léa",
        ],
    );
    let b = step(
        &a,
        "b.pdf",
        &[
            "--kind",
            "arrow",
            "--rect",
            "50,300,250,350",
            "--width",
            "3",
        ],
    );
    let c = step(
        &b,
        "c.pdf",
        &[
            "--kind",
            "ink",
            "--stroke",
            "300,300 320,340 340,300",
            "--stroke",
            "300,280 340,280",
        ],
    );
    let d = step(
        &c,
        "d.pdf",
        &[
            "--kind",
            "freetext",
            "--rect",
            "300,500,450,520",
            "--text",
            "Bonjour tout le monde, voici un texte qui passe à la ligne",
            "--size",
            "14",
            "--opacity",
            "1",
        ],
    );
    let e = step(
        &d,
        "e.pdf",
        &[
            "--kind",
            "link",
            "--rect",
            "72,600,200,630",
            "--goto-page",
            "2",
        ],
    );
    let out = stdout_json(
        &cli()
            .arg("annotate")
            .arg(&e)
            .args([
                "--kind",
                "link",
                "--rect",
                "300,600,400,630",
                "--url",
                "https://example.org",
            ])
            .arg("--out")
            .arg(dir.join("f.pdf"))
            .output()
            .unwrap(),
    );
    assert_eq!(
        (
            out["kind"].as_str(),
            out["index"].as_u64(),
            out["annotation_count"].as_u64()
        ),
        (Some("link"), Some(5), Some(6))
    );

    let list = listed(&dir.join("f.pdf"), None);
    let kinds: Vec<&str> = list
        .iter()
        .map(|a| a["subtype"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["Square", "Line", "Ink", "FreeText", "Link", "Link"]);
    assert_eq!(list[0]["author"], "Léa");
    assert_eq!(list[0]["color"], "#0000ff");
    assert_eq!(list[0]["fill"], "#ffff00");
    assert_eq!(list[0]["ours"], true);
    assert_eq!(
        list[0]["rect"],
        serde_json::json!([100.0, 100.0, 200.0, 160.0])
    );
    assert_eq!(list[1]["arrow"], true);
    assert_eq!(list[3]["font_size"], 14.0);
    assert_eq!(list[3]["opacity"], 1.0);
    assert!(list[3]["rect"][3].as_f64().unwrap() == 520.0);
    assert!(list[3]["contents"].as_str().unwrap().starts_with("Bonjour"));
    assert_eq!(list[4]["link"], serde_json::json!({ "page": 2 }));
    assert_eq!(
        list[5]["link"],
        serde_json::json!({ "uri": "https://example.org" })
    );

    // Sans --page : toutes les pages.
    let all = stdout_json(
        &cli()
            .arg("annotations")
            .arg(dir.join("f.pdf"))
            .output()
            .unwrap(),
    );
    assert_eq!(all["pages"].as_array().unwrap().len(), 2);
    assert!(all["pages"][1]["annotations"]
        .as_array()
        .unwrap()
        .is_empty());

    // Réponse, statut, suppression du fil.
    let r = dir.join("r.pdf");
    let o = cli()
        .arg("reply")
        .arg(dir.join("f.pdf"))
        .args(["--index", "0", "--text", "Bien reçu", "--author", "Max"])
        .arg("--out")
        .arg(&r)
        .output()
        .unwrap();
    assert_eq!(stdout_json(&o)["index"], 6);
    let s = dir.join("s.pdf");
    let o = cli()
        .arg("review-state")
        .arg(&r)
        .args(["--index", "0", "--state", "accepted", "--author", "Max"])
        .arg("--out")
        .arg(&s)
        .output()
        .unwrap();
    assert_eq!(stdout_json(&o)["state"], "accepted");
    let list = listed(&s, None);
    assert_eq!(list.len(), 8);
    assert_eq!(list[6]["parent"], 0);
    assert_eq!(list[6]["author"], "Max");
    assert_eq!(list[7]["state"], "accepted");
    assert_eq!(list[0]["replies"], 2);
    let t = dir.join("t.pdf");
    let o = cli()
        .arg("annotation-delete")
        .arg(&s)
        .args(["--index", "0", "--thread"])
        .arg("--out")
        .arg(&t)
        .output()
        .unwrap();
    assert_eq!(stdout_json(&o)["deleted"], 3);
    assert_eq!(listed(&t, None).len(), 5);
    let u = dir.join("u.pdf");
    let o = cli()
        .arg("annotation-delete")
        .arg(&t)
        .args(["--index", "0"])
        .arg("--out")
        .arg(&u)
        .output()
        .unwrap();
    assert_eq!(stdout_json(&o)["deleted"], 1);
    assert_eq!(listed(&u, None).len(), 4);

    // Erreurs typées : adresse dangereuse, rectangle absent ou mal formé, genre inconnu, statut
    // inconnu, page absente, sortie existante (code 6, sans --overwrite).
    let fail = |args: &[&str], code: i32, error: &str| {
        let o = cli()
            .arg("annotate")
            .arg(&base)
            .args(args)
            .arg("--out")
            .arg(dir.join("never.pdf"))
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(code), "{args:?}");
        assert_eq!(stderr_json(&o)["error"], error, "{args:?}");
        assert!(!dir.join("never.pdf").exists());
    };
    fail(
        &[
            "--kind",
            "link",
            "--rect",
            "10,10,60,40",
            "--url",
            "file:///C:/Windows/System32/calc.exe",
        ],
        1,
        "other",
    );
    fail(
        &[
            "--kind",
            "link",
            "--rect",
            "10,10,60,40",
            "--url",
            "javascript:alert(1)",
        ],
        1,
        "other",
    );
    fail(&["--kind", "link", "--rect", "10,10,60,40"], 1, "other");
    fail(&["--kind", "square"], 1, "other");
    fail(&["--kind", "square", "--rect", "1,2,3"], 1, "other");
    fail(&["--kind", "blob", "--rect", "1,2,30,40"], 1, "other");
    fail(&["--kind", "line", "--rect", "5,5,5.2,5.2"], 1, "other");
    fail(&["--kind", "ink"], 1, "other");
    fail(
        &["--kind", "square", "--rect", "10,10,60,40", "--page", "9"],
        1,
        "page_out_of_range",
    );
    fail(
        &["--kind", "freetext", "--rect", "10,10,160,40"],
        1,
        "other",
    );
    let o = cli()
        .arg("review-state")
        .arg(&base)
        .args(["--index", "0", "--state", "maybe"])
        .arg("--out")
        .arg(dir.join("never.pdf"))
        .output()
        .unwrap();
    assert_eq!(stderr_json(&o)["error"], "other");
    let o = cli()
        .arg("annotate")
        .arg(&base)
        .args(["--kind", "square", "--rect", "10,10,60,40"])
        .arg("--out")
        .arg(dir.join("a.pdf"))
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(6));
    assert_eq!(stderr_json(&o)["error"], "already_exists");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn annotating_an_encrypted_document_keeps_its_password() {
    let dir = std::env::temp_dir().join(format!("sunipdf-cli-annot-enc-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("enc.pdf");
    let o = cli()
        .arg("annotate")
        .arg(fixture("encrypted-rc4.pdf"))
        .args([
            "--kind",
            "circle",
            "--rect",
            "100,100,180,180",
            "--password",
            "secret",
        ])
        .arg("--out")
        .arg(&out)
        .output()
        .unwrap();
    stdout_json(&o);
    let o = cli().arg("annotations").arg(&out).output().unwrap();
    assert_eq!(o.status.code(), Some(3));
    assert_eq!(stderr_json(&o)["error"], "password_required");
    let list = listed(&out, Some("secret"));
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["subtype"], "Circle");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn foreign_annotations_are_listed_with_their_thread() {
    let list = listed(&fixture("annotated.pdf"), None);
    assert_eq!(list.len(), 12);
    assert_eq!(list[4]["replies"], 2);
    assert_eq!(list[5]["parent"], 4);
    assert_eq!(list[6]["state"], "accepted");
    assert_eq!(
        list[7]["link"],
        serde_json::json!({ "uri": "https://example.com" })
    );
    assert_eq!(list[10]["link"], serde_json::json!({ "unsupported": true }));
    assert!(list.iter().all(|a| a["ours"] == false));
}

fn rejects_without_writing(command: &str, args: &[&str], message: &str, name: &str) {
    let dir =
        std::env::temp_dir().join(format!("sunipdf-cli-invalid-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let base = fixture("two-pages.pdf");
    let original = std::fs::read(&base).unwrap();
    for overwrite in [false, true] {
        let out = dir.join(if overwrite { "existing.pdf" } else { "new.pdf" });
        if overwrite {
            std::fs::write(&out, &original).unwrap();
        }
        let input = if overwrite { &out } else { &base };
        let mut c = cli();
        c.arg(command).arg(input).args(args).arg("--out").arg(&out);
        if overwrite {
            c.arg("--overwrite");
        }
        let result = c.output().unwrap();
        assert_eq!(
            result.status.code(),
            Some(1),
            "{command} {args:?}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let error = stderr_json(&result);
        assert_eq!(error["error"], "other", "{command} {args:?}");
        assert!(
            error["message"].as_str().unwrap().contains(message),
            "{error}"
        );
        assert!(result.stdout.is_empty());
        if overwrite {
            assert_eq!(
                std::fs::read(&out).unwrap(),
                original,
                "rejection must preserve an existing input/output PDF"
            );
        } else {
            assert!(!out.exists(), "rejection must not create an output PDF");
        }
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn free_text_rejects_unsupported_opacity_without_writing_output() {
    rejects_without_writing(
        "annotate",
        &[
            "--kind",
            "freetext",
            "--rect",
            "10,10,160,40",
            "--text",
            "Rejected",
            "--opacity",
            "0.5",
        ],
        "free-text annotations only support --opacity 1",
        "freetext-opacity",
    );
}

#[test]
fn annotation_opacity_rejects_nonfinite_values_without_writing_output() {
    for value in ["NaN", "inf", "-inf"] {
        let opacity = format!("--opacity={value}");
        rejects_without_writing(
            "annotate",
            &["--kind", "square", "--rect", "10,10,160,40", &opacity],
            "opacity must be finite",
            "nonfinite-opacity",
        );
    }
}

#[test]
fn nonfinite_text_coordinates_and_multiline_stamps_preserve_output() {
    rejects_without_writing(
        "add-text",
        &["--x", "NaN", "--y", "72", "--text", "Rejected"],
        "invalid text coordinates",
        "nonfinite-text",
    );
    rejects_without_writing(
        "stamp",
        &["--text", "Line one\nLine two"],
        "stamp text must be a single line",
        "multiline-stamp",
    );
}
