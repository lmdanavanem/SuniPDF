//! Valeurs de formulaire ↔ JSON (`{ "nom": valeur }`) et coercition d'un texte (CLI `--set`).
//! Fonctions pures, sans PDFium : l'import est planifié sur un [`FormInfo`] déjà lu et chaque
//! valeur passe par [`FormField::validate`], si bien que `fill_form(.., Strict)` ne peut plus échouer.

use crate::forms::{FieldKind, FieldValue, FormField, FormInfo};
use crate::EngineError;
use serde_json::{json, Value};

/// Valeur JSON d'un champ : texte → chaîne, case → booléen, radio et liste déroulante → chaîne ou
/// `null`, zone de liste à choix multiple → tableau, bouton et signature → `null`.
pub fn value_to_json(f: &FormField) -> Value {
    match &f.value {
        FieldValue::None => Value::Null,
        FieldValue::Text(s) => Value::String(s.clone()),
        FieldValue::Bool(b) => Value::Bool(*b),
        FieldValue::Choice(v) if f.kind == FieldKind::List && f.multi_select => {
            Value::Array(v.iter().map(|s| Value::String(s.clone())).collect())
        }
        FieldValue::Choice(v) => v.first().map_or(Value::Null, |s| Value::String(s.clone())),
    }
}

/// `{ nom: valeur }` des champs texte, case, radio et listes (lecture seule comprise : l'import
/// accepte la valeur courante d'un tel champ) ; jamais un mot de passe ; homonymes : le premier gagne.
pub fn values_to_json(info: &FormInfo) -> Value {
    let mut map = serde_json::Map::new();
    for f in info
        .fields
        .iter()
        .filter(|f| f.kind.fillable() && !f.password)
    {
        map.entry(f.name.clone())
            .or_insert_with(|| value_to_json(f));
    }
    Value::Object(map)
}

/// Structure complète (CLI `form-fields`) ; pages en base 1, rectangles en espace utilisateur.
pub fn info_to_json(info: &FormInfo) -> Value {
    json!({
        "kind": info.kind.id(),
        "scripts": info.has_scripts,
        "count": info.fields.len(),
        "fields": info.fields.iter().map(|f| json!({
            "name": f.name,
            "type": f.kind.id(),
            "value": if f.password { Value::Null } else { value_to_json(f) },
            "read_only": f.read_only,
            "required": f.required,
            "tooltip": f.tooltip,
            "multiline": f.multiline,
            "password": f.password,
            "comb": f.comb,
            "max_length": f.max_len,
            "multi_select": f.multi_select,
            "editable": f.editable,
            "no_toggle_off": f.no_toggle_off,
            "options": f.options.iter().map(|o| json!({ "value": o.value, "label": o.label })).collect::<Vec<_>>(),
            "widgets": f.widgets.iter().map(|w| json!({
                "page": w.page + 1,
                "rect": [w.rect.left, w.rect.bottom, w.rect.right, w.rect.top],
                "on_state": w.on_state,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

#[derive(Debug, Default, PartialEq)]
pub struct ImportPlan {
    pub values: Vec<(String, FieldValue)>,
    pub skipped_unknown: Vec<String>,
    /// (nom, raison en anglais).
    pub skipped_invalid: Vec<(String, String)>,
}

fn parse_bool(s: &str) -> Option<bool> {
    match s.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "oui" | "1" => Some(true),
        "false" | "no" | "off" | "non" | "0" | "" => Some(false),
        _ => None,
    }
}

fn json_strings(v: &Value) -> Result<Vec<String>, String> {
    match v {
        Value::Null => Ok(Vec::new()),
        Value::String(s) if s.is_empty() => Ok(Vec::new()),
        Value::String(s) => Ok(vec![s.clone()]),
        Value::Number(n) => Ok(vec![n.to_string()]),
        Value::Array(a) => a
            .iter()
            .map(|x| match x {
                Value::String(s) => Ok(s.clone()),
                Value::Number(n) => Ok(n.to_string()),
                other => Err(format!("unsupported choice value: {other}")),
            })
            .collect(),
        other => Err(format!("unsupported choice value: {other}")),
    }
}

/// Valeur JSON → valeur typée selon le champ (`null` = vide ; nombres acceptés comme texte).
fn json_to_value(f: &FormField, v: &Value) -> Result<FieldValue, String> {
    Ok(match f.kind {
        FieldKind::Text => FieldValue::Text(match v {
            Value::Null => String::new(),
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            other => return Err(format!("unsupported text value: {other}")),
        }),
        FieldKind::Checkbox => FieldValue::Bool(match v {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
            Value::String(s) => parse_bool(s).ok_or_else(|| format!("not a boolean: {s:?}"))?,
            other => return Err(format!("unsupported checkbox value: {other}")),
        }),
        FieldKind::Radio | FieldKind::Combo | FieldKind::List => {
            FieldValue::Choice(json_strings(v)?)
        }
        FieldKind::PushButton | FieldKind::Signature => FieldValue::None,
    })
}

/// Plan d'import : chaque entrée de l'objet JSON est rattachée à son champ et validée ; un JSON
/// qui n'est pas un objet → `FormValueInvalid`. Un champ en lecture seule qui reçoit sa valeur
/// courante (fichier exporté puis réimporté) n'est ni appliqué ni compté comme ignoré.
pub fn plan_from_json(info: &FormInfo, json: &Value) -> Result<ImportPlan, EngineError> {
    let Value::Object(map) = json else {
        return Err(EngineError::FormValueInvalid(
            "a JSON object {\"field name\": value} is expected".into(),
        ));
    };
    let mut plan = ImportPlan::default();
    for (name, v) in map {
        let Some(f) = info.field(name) else {
            plan.skipped_unknown.push(name.clone());
            continue;
        };
        match json_to_value(f, v).and_then(|value| f.validate(&value).map(|_| value)) {
            Ok(_) if f.read_only => {}
            Ok(value) => plan.values.push((name.clone(), value)),
            Err(reason) => plan.skipped_invalid.push((name.clone(), reason)),
        }
    }
    Ok(plan)
}

impl FormField {
    /// Texte de ligne de commande → valeur : case = booléen, radio / liste déroulante = un choix
    /// (`""` = aucun), zone de liste = choix séparés par `;`. Raison de refus en anglais, sans le
    /// nom du champ (l'appelant le place).
    pub fn coerce_text(&self, text: &str) -> Result<FieldValue, String> {
        let value = match self.kind {
            FieldKind::Text => FieldValue::Text(text.to_string()),
            FieldKind::Checkbox => FieldValue::Bool(
                parse_bool(text).ok_or_else(|| format!("not a boolean: {text:?}"))?,
            ),
            FieldKind::Radio | FieldKind::Combo => FieldValue::Choice(if text.is_empty() {
                Vec::new()
            } else {
                vec![text.to_string()]
            }),
            FieldKind::List => FieldValue::Choice(
                text.split(';')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect(),
            ),
            FieldKind::PushButton | FieldKind::Signature => FieldValue::None,
        };
        self.validate(&value)?;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::FieldOption;

    fn field(name: &str, kind: FieldKind) -> FormField {
        FormField {
            name: name.into(),
            kind,
            value: FieldValue::None,
            tooltip: None,
            read_only: false,
            required: false,
            multiline: false,
            password: false,
            comb: false,
            max_len: None,
            multi_select: false,
            editable: false,
            no_toggle_off: false,
            options: ["FR", "BE"]
                .iter()
                .map(|v| FieldOption {
                    value: v.to_string(),
                    label: format!("{v}-label"),
                })
                .collect(),
            widgets: Vec::new(),
        }
    }

    fn info() -> FormInfo {
        FormInfo {
            kind: crate::FormKind::AcroForm,
            has_scripts: false,
            fields: vec![
                FormField {
                    value: FieldValue::Text("Élodie".into()),
                    ..field("nom", FieldKind::Text)
                },
                FormField {
                    value: FieldValue::Bool(false),
                    ..field("ok", FieldKind::Checkbox)
                },
                FormField {
                    value: FieldValue::Choice(vec!["BE".into()]),
                    ..field("pays", FieldKind::Combo)
                },
                FormField {
                    multi_select: true,
                    value: FieldValue::Choice(vec!["FR".into(), "BE".into()]),
                    ..field("langues", FieldKind::List)
                },
                FormField {
                    read_only: true,
                    value: FieldValue::Text("REF".into()),
                    ..field("ref", FieldKind::Text)
                },
                field("envoyer", FieldKind::PushButton),
                FormField {
                    password: true,
                    value: FieldValue::Text("hunter2".into()),
                    ..field("secret", FieldKind::Text)
                },
            ],
        }
    }

    #[test]
    fn export_and_import_round_trip() {
        let info = info();
        let v = values_to_json(&info);
        assert_eq!(v["nom"], "Élodie");
        assert_eq!(v["ok"], false);
        assert_eq!(v["pays"], "BE");
        assert_eq!(v["langues"], json!(["FR", "BE"]));
        assert_eq!(v["ref"], "REF");
        assert!(v.get("envoyer").is_none(), "bouton omis");
        assert!(v.get("secret").is_none(), "mot de passe jamais exporté");
        // Export → import tel quel : rien d'ignoré, la lecture seule n'est pas réécrite.
        let same = plan_from_json(&info, &v).unwrap();
        assert!(same.skipped_unknown.is_empty() && same.skipped_invalid.is_empty());
        assert_eq!(same.values.len(), 4);
        assert!(same.values.iter().all(|(n, _)| n != "ref"));
        let plan = plan_from_json(
            &info,
            &json!({
                "nom": "Zoé", "ok": "oui", "pays": "FR-label", "langues": ["FR"],
                "inconnu": 1, "ref": "x", "envoyer": null, "ok2": 3
            }),
        )
        .unwrap();
        // Objet JSON trié par clé (serde_json sans `preserve_order`) : lecture par nom.
        assert_eq!(plan.values.len(), 4);
        let planned = |n: &str| {
            plan.values
                .iter()
                .find(|(k, _)| k == n)
                .map(|(_, v)| v.clone())
        };
        assert_eq!(planned("ok"), Some(FieldValue::Bool(true)));
        assert_eq!(
            planned("pays"),
            Some(FieldValue::Choice(vec!["FR-label".into()]))
        );
        assert_eq!(planned("nom"), Some(FieldValue::Text("Zoé".into())));
        assert_eq!(plan.skipped_unknown, vec!["inconnu", "ok2"]);
        let mut invalid: Vec<&str> = plan
            .skipped_invalid
            .iter()
            .map(|(n, _)| n.as_str())
            .collect();
        invalid.sort_unstable();
        assert_eq!(invalid, vec!["envoyer", "ref"]);
        assert!(matches!(
            plan_from_json(&info, &json!([1, 2])),
            Err(EngineError::FormValueInvalid(_))
        ));
        let full = info_to_json(&info);
        assert_eq!(full["count"], 7);
        assert_eq!(full["fields"][3]["multi_select"], true);
        assert_eq!(full["fields"][2]["options"][1]["label"], "BE-label");
    }

    #[test]
    fn coerce_text_per_kind() {
        let info = info();
        let f = |n: &str| info.field(n).unwrap();
        assert_eq!(f("ok").coerce_text("Yes").unwrap(), FieldValue::Bool(true));
        assert!(f("ok").coerce_text("peut-être").is_err());
        assert_eq!(
            f("langues").coerce_text("FR; BE").unwrap(),
            FieldValue::Choice(vec!["FR".into(), "BE".into()])
        );
        assert_eq!(
            f("pays").coerce_text("").unwrap(),
            FieldValue::Choice(vec![])
        );
        assert_eq!(
            f("pays").coerce_text("XX"),
            Err("unknown option: XX".into()),
            "raison sans le nom du champ"
        );
        assert!(f("ref").coerce_text("x").is_err(), "lecture seule");
        assert!(f("ref").coerce_text("REF").is_ok(), "valeur courante");
        assert!(f("secret").coerce_text("x").is_err(), "mot de passe");
        assert_eq!(
            f("nom").coerce_text("Jean").unwrap(),
            FieldValue::Text("Jean".into())
        );
    }
}
