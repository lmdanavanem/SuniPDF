//! `form-fields`, `fill-form`, `flatten` de bout en bout (binaire lancé, fichiers rouverts) : exige
//! `pdfium.dll` (scripts/fetch-pdfium.ps1), comme les tests d'intégration du moteur.

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

fn fields(path: &Path) -> serde_json::Value {
    stdout_json(&cli().arg("form-fields").arg(path).output().unwrap())
}

fn value_of<'a>(info: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    let f = info["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == name)
        .unwrap_or_else(|| panic!("champ {name}"));
    &f["value"]
}

#[test]
fn form_fields_fill_form_and_flatten_round_trip() {
    let dir = std::env::temp_dir().join(format!("sunipdf-cli-forms-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let form = fixture("acroform.pdf");

    // Liste complète : 13 champs, pages en base 1, options de la liste déroulante.
    let info = fields(&form);
    assert_eq!(info["kind"], "acroform");
    assert_eq!(info["count"], 13);
    assert_eq!(info["scripts"], false);
    assert_eq!(value_of(&info, "ville"), "Élodie Müller");
    assert_eq!(value_of(&info, "paiement"), "virement");
    assert_eq!(value_of(&info, "langues"), &serde_json::json!(["Anglais"]));
    let pays = info["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "pays")
        .unwrap();
    assert_eq!(pays["type"], "combo");
    assert_eq!(pays["options"][1]["label"], "Belgique");
    assert_eq!(pays["widgets"][0]["page"], 1);
    assert_eq!(
        pays["widgets"][0]["rect"],
        serde_json::json!([150.0, 436.0, 350.0, 456.0])
    );

    // Valeurs seules : le format accepté par --values.
    let o = cli()
        .arg("form-fields")
        .arg(&form)
        .arg("--values-only")
        .output()
        .unwrap();
    let values = stdout_json(&o);
    assert_eq!(values["pays"], "BE");
    assert!(values.get("signature").is_none());

    // --set, puis relecture dans le fichier produit.
    let out = dir.join("filled.pdf");
    let o = cli()
        .arg("fill-form")
        .arg(&form)
        .args([
            "--set",
            "nom=Jean",
            "--set",
            "accepte=true",
            "--set",
            "langues=Français;Allemand",
        ])
        .arg("--out")
        .arg(&out)
        .output()
        .unwrap();
    let v = stdout_json(&o);
    assert_eq!(v["applied"], 3);
    assert_eq!(v["flattened"], false);
    assert_eq!(v["skipped"]["unknown"], serde_json::json!([]));
    let info = fields(&out);
    assert_eq!(value_of(&info, "nom"), "Jean");
    assert_eq!(value_of(&info, "accepte"), true);
    assert_eq!(
        value_of(&info, "langues"),
        &serde_json::json!(["Français", "Allemand"])
    );

    // Fichier de valeurs JSON (libellé de liste accepté), puis aplatissement : plus de champ.
    let json = dir.join("values.json");
    std::fs::write(
        &json,
        r#"{ "pays": "Canada", "paiement": "carte", "ville": "Zoë" }"#,
    )
    .unwrap();
    let flat = dir.join("flat.pdf");
    let o = cli()
        .arg("fill-form")
        .arg(&out)
        .arg("--values")
        .arg(&json)
        .arg("--flatten")
        .arg("--out")
        .arg(&flat)
        .output()
        .unwrap();
    let v = stdout_json(&o);
    assert_eq!(v["applied"], 3);
    assert_eq!(v["flattened"], true);
    let info = fields(&flat);
    assert_eq!(info["kind"], "none");
    assert_eq!(info["count"], 0);
    // Le rendu du fichier aplati montre encore le texte saisi (pixels sombres dans « nom »).
    let png = dir.join("flat.png");
    let o = cli()
        .arg("render")
        .arg(&flat)
        .args(["--page", "1", "--scale", "1", "--out"])
        .arg(&png)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let page = image::open(&png).unwrap().into_rgba8();
    let dark = (150..410)
        .flat_map(|x| (792 - 716..792 - 696).map(move |y| (x, y)))
        .filter(|&(x, y)| {
            let p = page.get_pixel(x, y).0;
            p[0] < 100 && p[1] < 100 && p[2] < 100
        })
        .count();
    assert!(dark > 300, "{dark} pixels sombres dans le champ nom aplati");

    // Champ inconnu : refus strict (code 1), rien n'est écrit ; tolérant avec --ignore-unknown.
    let out2 = dir.join("strict.pdf");
    let o = cli()
        .arg("fill-form")
        .arg(&form)
        .args(["--set", "inconnu=1", "--out"])
        .arg(&out2)
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(stderr_json(&o)["error"], "form_field_not_found");
    assert!(!out2.exists());
    let o = cli()
        .arg("fill-form")
        .arg(&form)
        .args([
            "--set",
            "inconnu=1",
            "--set",
            "ref=x",
            "--ignore-unknown",
            "--out",
        ])
        .arg(&out2)
        .output()
        .unwrap();
    let v = stdout_json(&o);
    assert_eq!(v["applied"], 0);
    assert_eq!(v["skipped"]["unknown"], serde_json::json!(["inconnu"]));
    assert_eq!(v["skipped"]["invalid"][0]["field"], "ref");
    assert!(out2.exists());

    // Valeur refusée (lecture seule) en mode strict.
    let o = cli()
        .arg("fill-form")
        .arg(&form)
        .args(["--set", "ref=x", "--out"])
        .arg(dir.join("ro.pdf"))
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(stderr_json(&o)["error"], "form_value_invalid");

    // `flatten` seul, puis sortie existante refusée sans --overwrite (code 6).
    let flat2 = dir.join("flat2.pdf");
    let o = cli()
        .arg("flatten")
        .arg(&form)
        .arg("--out")
        .arg(&flat2)
        .output()
        .unwrap();
    let v = stdout_json(&o);
    assert!(v["annotations"].as_u64().unwrap() >= 16);
    assert_eq!(v["pages"], 1);
    assert_eq!(fields(&flat2)["count"], 0);
    let o = cli()
        .arg("flatten")
        .arg(&form)
        .arg("--out")
        .arg(&flat2)
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(6));
    assert_eq!(stderr_json(&o)["error"], "already_exists");

    // XFA : lecture seule.
    let o = cli()
        .arg("fill-form")
        .arg(fixture("acroform-xfa.pdf"))
        .args(["--set", "nom=x", "--out"])
        .arg(dir.join("xfa.pdf"))
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(stderr_json(&o)["error"], "form_xfa");
    assert_eq!(fields(&fixture("acroform-xfa.pdf"))["kind"], "xfa");

    // Document sans formulaire : liste vide, aplatissement sans erreur.
    let plain = fields(&fixture("two-pages.pdf"));
    assert_eq!(plain["kind"], "none");
    assert_eq!(plain["count"], 0);
    let _ = std::fs::remove_dir_all(&dir);
}
