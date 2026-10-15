//! `redact` de bout en bout (binaire lancé, fichier relu par `search`) : exige `pdfium.dll`.
//! L'absence des octets est prouvée par les tests du moteur (`redaction_*`) ; ici, le contrat CLI.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"))
}

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/sunipdf-core/tests/fixtures/redact.pdf")
}

fn json(bytes: &[u8]) -> serde_json::Value {
    serde_json::from_slice(bytes).unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(bytes)))
}

fn search(file: &Path, query: &str) -> u64 {
    let o = cli().arg("search").arg(file).arg(query).output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    json(&o.stdout)["count"].as_u64().unwrap()
}

fn redact(args: &[&str], out: &Path) -> Output {
    cli()
        .arg("redact")
        .arg(fixture())
        .args(args)
        .arg("--out")
        .arg(out)
        .output()
        .unwrap()
}

#[test]
fn redact_text_and_rectangle_then_reread() {
    let dir = std::env::temp_dir().join(format!("sunipdf-cli-redact-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("out.pdf");
    assert_eq!(search(&fixture(), "SECRET-ALPHA-7731"), 1);

    let o = redact(
        &["--text", "SECRET-ALPHA-7731", "--rect", "1:72,440,272,460"],
        &out,
    );
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let r = json(&o.stdout);
    assert_eq!(r["marks"], 2);
    assert_eq!(r["pages"], 1);
    assert_eq!(r["fields_removed"], 1);
    assert_eq!(r["remaining_matches"], 0);
    assert_eq!(r["hidden_data_cleaned"], true);
    assert_eq!(search(&out, "SECRET-ALPHA"), 0);
    assert_eq!(search(&out, "KEEPME-42"), 1, "le reste du texte est intact");

    // Sortie existante : refus sans écraser (code 6).
    let o = redact(&["--text", "KEEPME-42"], &out);
    assert_eq!(o.status.code(), Some(6));
    assert_eq!(json(&o.stderr)["error"], "already_exists");

    // Rien à caviarder : erreur, aucun fichier.
    let none = dir.join("none.pdf");
    let o = redact(&["--text", "NO-SUCH-TEXT"], &none);
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(json(&o.stderr)["error"], "other");
    assert!(!none.exists());

    // Rectangle mal formé ; page hors du document.
    let o = redact(&["--rect", "1:72,440,272"], &none);
    assert_eq!(o.status.code(), Some(1));
    let o = redact(&["--rect", "5:72,440,272,460"], &none);
    assert_eq!(json(&o.stderr)["error"], "page_out_of_range");
    assert!(!none.exists());

    // Mode page en image : plus aucun texte.
    let raster = dir.join("raster.pdf");
    let o = redact(&["--rect", "1:500,100,520,120", "--rasterize"], &raster);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(json(&o.stdout)["pages_rasterized"], 1);
    assert_eq!(search(&raster, "KEEPME-42"), 0);
    let _ = std::fs::remove_dir_all(dir);
}
