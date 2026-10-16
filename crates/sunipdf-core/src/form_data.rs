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
            Value::String(s) => parse_bool(s)
                .or_else(|| {
                    f.widgets
                        .iter()
                        .any(|w| w.on_state.as_deref() == Some(s))
                        .then_some(true)
                })
                .ok_or_else(|| format!("not a boolean or checkbox export state: {s:?}"))?,
            other => return Err(format!("unsupported checkbox value: {other}")),
        }),
        FieldKind::Radio if v.as_str() == Some("Off") => FieldValue::Choice(Vec::new()),
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

/// Refuses the complete import if any field or value is invalid; callers apply this plan once.
pub fn strict_plan_from_json(info: &FormInfo, json: &Value) -> Result<ImportPlan, EngineError> {
    let plan = plan_from_json(info, json)?;
    if let Some(name) = plan.skipped_unknown.first() {
        return Err(EngineError::FormFieldNotFound(name.clone()));
    }
    if let Some((name, reason)) = plan.skipped_invalid.first() {
        return Err(import_error(format!("{name}: {reason}")));
    }
    Ok(plan)
}

pub const IMPORT_LIMIT: usize = 16 * 1024 * 1024;
const XFDF_NS: &str = "http://ns.adobe.com/xfdf/";

fn import_error(reason: impl std::fmt::Display) -> EngineError {
    EngineError::FormValueInvalid(format!("form data: {reason}"))
}

/// Encodes values only. Passwords, actions and source file locations are never exported.
pub fn export_values(info: &FormInfo, extension: &str) -> Result<Vec<u8>, EngineError> {
    match extension.to_ascii_lowercase().as_str() {
        "json" | "" => serde_json::to_vec_pretty(&values_to_json(info)).map_err(import_error),
        "xfdf" => {
            use xml::writer::XmlEvent;
            let mut writer = xml::writer::EmitterConfig::new()
                .perform_indent(true)
                .create_writer(Vec::new());
            writer
                .write(XmlEvent::start_element("xfdf").default_ns(XFDF_NS))
                .map_err(import_error)?;
            writer
                .write(XmlEvent::start_element("fields"))
                .map_err(import_error)?;
            let mut names = std::collections::HashSet::new();
            for field in info
                .fields
                .iter()
                .filter(|f| f.kind.fillable() && !f.password)
            {
                if !xml_text(&field.name) {
                    return Err(import_error("field name contains invalid XML characters"));
                }
                if !names.insert(&field.name) {
                    return Err(import_error("duplicate PDF field names"));
                }
                writer
                    .write(XmlEvent::start_element("field").attr("name", &field.name))
                    .map_err(import_error)?;
                let values = match &field.value {
                    FieldValue::None => vec![String::new()],
                    FieldValue::Text(s) => vec![s.clone()],
                    FieldValue::Bool(false) => vec!["Off".into()],
                    FieldValue::Bool(true) => vec![field
                        .widgets
                        .iter()
                        .find_map(|w| w.on_state.clone())
                        .unwrap_or_else(|| "Yes".into())],
                    FieldValue::Choice(v) if v.is_empty() => vec![String::new()],
                    FieldValue::Choice(v) => v.clone(),
                };
                for value in values {
                    if !xml_text(&value) {
                        return Err(import_error("field value contains invalid XML characters"));
                    }
                    writer
                        .write(XmlEvent::start_element("value"))
                        .map_err(import_error)?;
                    writer
                        .write(XmlEvent::characters(&value))
                        .map_err(import_error)?;
                    writer
                        .write(XmlEvent::end_element())
                        .map_err(import_error)?;
                }
                writer
                    .write(XmlEvent::end_element())
                    .map_err(import_error)?;
            }
            writer
                .write(XmlEvent::end_element())
                .map_err(import_error)?;
            writer
                .write(XmlEvent::end_element())
                .map_err(import_error)?;
            Ok(writer.into_inner())
        }
        _ => Err(import_error("export requires .json or .xfdf")),
    }
}

fn xml_text(value: &str) -> bool {
    value.chars().all(|c| {
        matches!(c, '\t' | '\n' | '\r') || (c >= '\u{20}' && c != '\u{fffe}' && c != '\u{ffff}')
    })
}

/// Parses bounded value-only data; document references are metadata and are never opened.
pub fn parse_values(bytes: &[u8], extension: &str) -> Result<Value, EngineError> {
    if bytes.len() > IMPORT_LIMIT {
        return Err(import_error("16 MiB import limit exceeded"));
    }
    let value = match extension.to_ascii_lowercase().as_str() {
        "json" | "" => serde_json::from_slice(bytes).map_err(import_error)?,
        "xfdf" => parse_xfdf(bytes)?,
        "fdf" => parse_fdf(bytes)?,
        _ => return Err(import_error("import requires .json, .xfdf or .fdf")),
    };
    if !value.is_object() {
        return Err(import_error("a field name/value object is required"));
    }
    Ok(value)
}

fn parse_xfdf(bytes: &[u8]) -> Result<Value, EngineError> {
    use xml::reader::{ParserConfig2, XmlEvent};
    let mut reader = ParserConfig2::new()
        .ignore_comments(true)
        .trim_whitespace(false)
        .max_data_length(IMPORT_LIMIT)
        .max_attribute_length(4096)
        .max_entity_expansion_length(4096)
        .create_reader(bytes);
    let mut stack: Vec<String> = Vec::new();
    let mut fields: Vec<(String, Vec<String>, bool)> = Vec::new();
    let mut values = serde_json::Map::new();
    let mut text = String::new();
    let mut root = false;
    let mut fields_seen = false;
    loop {
        let event = reader.next().map_err(import_error)?;
        if reader.doctype().is_some() {
            return Err(import_error("DTD declarations are not supported"));
        }
        match event {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                if stack.len() >= 32 {
                    return Err(import_error("XML nesting limit exceeded"));
                }
                if name.namespace.as_deref() != Some(XFDF_NS) {
                    return Err(import_error("unsupported XFDF namespace"));
                }
                let tag = name.local_name;
                let parent = stack.last().map(String::as_str);
                let allowed = match (parent, tag.as_str()) {
                    (None, "xfdf") if !root => {
                        root = true;
                        true
                    }
                    (Some("xfdf"), "fields") if !fields_seen => {
                        fields_seen = true;
                        true
                    }
                    (Some("xfdf"), "f" | "ids") => true,
                    (Some("fields" | "field"), "field") => true,
                    (Some("field"), "value") => true,
                    _ => false,
                };
                if !allowed {
                    return Err(import_error(format!("unsupported XFDF element: {tag}")));
                }
                for attribute in &attributes {
                    let valid = match (tag.as_str(), attribute.name.local_name.as_str()) {
                        ("field", "name") | ("f", "href") | ("ids", "original" | "modified") => {
                            attribute.name.namespace.is_none()
                        }
                        ("xfdf" | "value", "space") => {
                            attribute.name.namespace.as_deref()
                                == Some("http://www.w3.org/XML/1998/namespace")
                        }
                        _ => false,
                    };
                    if !valid || attribute.value.contains('\0') {
                        return Err(import_error("unsupported XFDF attribute"));
                    }
                }
                if tag == "field" {
                    let name = attributes
                        .iter()
                        .find(|a| a.name.local_name == "name")
                        .map(|a| a.value.as_str())
                        .ok_or_else(|| import_error("field name missing"))?;
                    if name.is_empty() || name.len() > 4096 || name.chars().any(char::is_control) {
                        return Err(import_error("invalid field name"));
                    }
                    let qualified =
                        if let Some((parent, parent_values, has_children)) = fields.last_mut() {
                            if !parent_values.is_empty() {
                                return Err(import_error(
                                    "a field cannot contain both a value and child fields",
                                ));
                            }
                            *has_children = true;
                            format!("{parent}.{name}")
                        } else {
                            name.to_owned()
                        };
                    fields.push((qualified, Vec::new(), false));
                } else if tag == "value" {
                    if fields.last().is_some_and(|(_, _, children)| *children) {
                        return Err(import_error(
                            "a field cannot contain both a value and child fields",
                        ));
                    }
                    text.clear();
                }
                stack.push(tag);
            }
            XmlEvent::Characters(s) | XmlEvent::CData(s) | XmlEvent::Whitespace(s) => {
                if stack.last().is_some_and(|x| x == "value") {
                    text.push_str(&s);
                } else if !s.trim().is_empty() {
                    return Err(import_error("text outside XFDF value"));
                }
            }
            XmlEvent::EndElement { .. } => match stack.pop().as_deref() {
                Some("value") => {
                    fields
                        .last_mut()
                        .ok_or_else(|| import_error("value outside field"))?
                        .1
                        .push(std::mem::take(&mut text));
                }
                Some("field") => {
                    let (name, field_values, children) = fields
                        .pop()
                        .ok_or_else(|| import_error("unbalanced field"))?;
                    if !children {
                        let value = match field_values.as_slice() {
                            [] => Value::Null,
                            [v] => Value::String(v.clone()),
                            _ => {
                                Value::Array(field_values.into_iter().map(Value::String).collect())
                            }
                        };
                        if values.insert(name, value).is_some() {
                            return Err(import_error("duplicate field name"));
                        }
                        if values.len() > 10000 {
                            return Err(import_error("field count limit exceeded"));
                        }
                    }
                }
                _ => {}
            },
            XmlEvent::ProcessingInstruction { .. } => {
                return Err(import_error("processing instructions are not supported"))
            }
            XmlEvent::EndDocument => break,
            _ => {}
        }
    }
    if !root || !fields_seen || !stack.is_empty() {
        return Err(import_error("incomplete XFDF document"));
    }
    Ok(Value::Object(values))
}

// shortcut: FDF imports only text/name/array values with a normal xref; add other FDF features only with interoperability fixtures.
fn parse_fdf(bytes: &[u8]) -> Result<Value, EngineError> {
    use lopdf::Object;
    if !bytes.starts_with(b"%FDF-") {
        return Err(import_error("FDF header missing"));
    }
    // Bound syntax before lopdf recurses; escaped strings and comments do not count as containers.
    let (mut depth, mut string_depth, mut hex, mut comment, mut escaped) =
        (0usize, 0usize, false, false, false);
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if comment {
            if matches!(b, b'\r' | b'\n') {
                comment = false;
            }
        } else if string_depth > 0 {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'(' {
                string_depth += 1;
            } else if b == b')' {
                string_depth -= 1;
            }
        } else if hex {
            if b == b'>' {
                hex = false;
            }
        } else {
            match b {
                b'%' => comment = true,
                b'(' => string_depth = 1,
                b's' if i > 0
                    && (bytes[i - 1].is_ascii_whitespace()
                        || matches!(bytes[i - 1], b'>' | b']' | b')'))
                    && bytes[i..].starts_with(b"stream")
                    && bytes.get(i + 6).is_some_and(u8::is_ascii_whitespace) =>
                {
                    return Err(import_error("stream-based FDF is not supported"));
                }
                b'<' if bytes.get(i + 1) == Some(&b'<') => {
                    depth += 1;
                    i += 1;
                }
                b'<' => hex = true,
                b'[' => depth += 1,
                b']' => {
                    depth = depth
                        .checked_sub(1)
                        .ok_or_else(|| import_error("unbalanced FDF syntax"))?
                }
                b'>' if bytes.get(i + 1) == Some(&b'>') => {
                    depth = depth
                        .checked_sub(1)
                        .ok_or_else(|| import_error("unbalanced FDF syntax"))?;
                    i += 1;
                }
                _ => {}
            }
        }
        if depth > 32 || string_depth > 32 {
            return Err(import_error("FDF nesting limit exceeded"));
        }
        i += 1;
    }
    if depth != 0 || string_depth != 0 || hex {
        return Err(import_error("unbalanced FDF syntax"));
    }
    let mut pdf_bytes = bytes.to_vec();
    pdf_bytes[1..4].copy_from_slice(b"PDF");
    let pdf = lopdf::Document::load_mem_with_options(
        &pdf_bytes,
        lopdf::LoadOptions::with_max_decompressed_size(IMPORT_LIMIT),
    )
    .map_err(|_| import_error("invalid FDF or missing normal cross-reference table"))?;
    if pdf.is_encrypted() || pdf.objects.values().any(|o| matches!(o, Object::Stream(_))) {
        return Err(import_error(
            "encrypted or stream-based FDF is not supported",
        ));
    }
    fn resolve<'a>(
        pdf: &'a lopdf::Document,
        mut object: &'a Object,
    ) -> Result<&'a Object, EngineError> {
        for _ in 0..32 {
            if let Object::Reference(id) = object {
                object = pdf.get_object(*id).map_err(import_error)?;
            } else {
                return Ok(object);
            }
        }
        Err(import_error("cyclic FDF object reference"))
    }
    fn check_keys(dictionary: &lopdf::Dictionary, allowed: &[&[u8]]) -> Result<(), EngineError> {
        if dictionary
            .iter()
            .any(|(key, _)| !allowed.contains(&key.as_slice()))
        {
            return Err(import_error(
                "unsupported FDF property, action or attachment",
            ));
        }
        Ok(())
    }
    fn scalar(pdf: &lopdf::Document, object: &Object) -> Result<Value, EngineError> {
        Ok(match resolve(pdf, object)? {
            Object::Null => Value::Null,
            Object::String(_, _) => Value::String(
                crate::forms::text_of(pdf, resolve(pdf, object)?)
                    .ok_or_else(|| import_error("invalid FDF text string"))?,
            ),
            Object::Name(name) => {
                Value::String(String::from_utf8(name.clone()).map_err(import_error)?)
            }
            _ => return Err(import_error("unsupported FDF field value")),
        })
    }
    fn walk(
        pdf: &lopdf::Document,
        objects: &[Object],
        prefix: &str,
        depth: usize,
        values: &mut serde_json::Map<String, Value>,
        visited: &mut std::collections::HashSet<lopdf::ObjectId>,
    ) -> Result<(), EngineError> {
        if depth > 32 {
            return Err(import_error("FDF field nesting limit exceeded"));
        }
        for object in objects {
            if let Object::Reference(id) = object {
                if !visited.insert(*id) {
                    return Err(import_error("duplicate or cyclic FDF field"));
                }
            }
            let field = resolve(pdf, object)?.as_dict().map_err(import_error)?;
            check_keys(field, &[b"T", b"V", b"Kids"])?;
            let part = match scalar(pdf, field.get(b"T").map_err(import_error)?)? {
                Value::String(s) => s,
                _ => return Err(import_error("FDF field name must be text")),
            };
            if part.is_empty() || part.len() > 4096 || part.chars().any(char::is_control) {
                return Err(import_error("invalid FDF field name"));
            }
            let name = if prefix.is_empty() {
                part
            } else {
                format!("{prefix}.{part}")
            };
            if let Ok(kids) = field.get(b"Kids") {
                if field.has(b"V") {
                    return Err(import_error("FDF field cannot have both Kids and V"));
                }
                walk(
                    pdf,
                    resolve(pdf, kids)?.as_array().map_err(import_error)?,
                    &name,
                    depth + 1,
                    values,
                    visited,
                )?;
            } else {
                let value = match field.get(b"V") {
                    Err(_) => Value::Null,
                    Ok(value) => match resolve(pdf, value)? {
                        Object::Array(a) => Value::Array(
                            a.iter().map(|o| scalar(pdf, o)).collect::<Result<_, _>>()?,
                        ),
                        value => scalar(pdf, value)?,
                    },
                };
                if values.insert(name, value).is_some() {
                    return Err(import_error("duplicate FDF field name"));
                }
                if values.len() > 10000 {
                    return Err(import_error("field count limit exceeded"));
                }
            }
        }
        Ok(())
    }
    let root = resolve(&pdf, pdf.trailer.get(b"Root").map_err(import_error)?)?
        .as_dict()
        .map_err(import_error)?;
    check_keys(root, &[b"FDF", b"Type"])?;
    let data = resolve(&pdf, root.get(b"FDF").map_err(import_error)?)?
        .as_dict()
        .map_err(import_error)?;
    check_keys(data, &[b"Fields", b"F", b"ID"])?;
    if let Ok(file) = data.get(b"F") {
        if !matches!(scalar(&pdf, file)?, Value::String(_)) {
            return Err(import_error("unsupported FDF file reference"));
        }
    }
    let fields = resolve(&pdf, data.get(b"Fields").map_err(import_error)?)?
        .as_array()
        .map_err(import_error)?;
    let mut values = serde_json::Map::new();
    walk(
        &pdf,
        fields,
        "",
        0,
        &mut values,
        &mut std::collections::HashSet::new(),
    )?;
    Ok(Value::Object(values))
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
    #[test]
    fn xfdf_round_trip_and_rejects_unsafe_or_ambiguous_data() {
        assert_eq!(
            json_to_value(&field("radio", FieldKind::Radio), &json!("Off")).unwrap(),
            FieldValue::Choice(Vec::new())
        );
        assert_eq!(
            json_to_value(&field("combo", FieldKind::Combo), &json!("Off")).unwrap(),
            FieldValue::Choice(vec!["Off".into()])
        );
        let info = info();
        let xml = export_values(&info, "xfdf").unwrap();
        let parsed = parse_values(&xml, "xfdf").unwrap();
        let plan = strict_plan_from_json(&info, &parsed).unwrap();
        assert_eq!(plan.values.len(), 4);
        assert!(String::from_utf8(xml).unwrap().contains("Élodie"));
        let nested = br#"<xfdf xmlns="http://ns.adobe.com/xfdf/"><f href="file:///never-open-this.pdf"/><fields><field name="person"><field name="name"><value>  A &amp; <![CDATA[B]]>  </value></field></field></fields></xfdf>"#;
        assert_eq!(
            parse_values(nested, "xfdf").unwrap(),
            json!({ "person.name": "  A & B  " })
        );
        for invalid in [
            "<!DOCTYPE xfdf [<!ENTITY stolen SYSTEM 'file:///secret'>]><xfdf xmlns='http://ns.adobe.com/xfdf/'><fields/></xfdf>",
            "<xfdf xmlns='http://ns.adobe.com/xfdf/'><fields><field name='a'><value>1</value></field><field name='a'><value>2</value></field></fields></xfdf>",
            "<xfdf xmlns='http://ns.adobe.com/xfdf/'><fields><field name='a'><value>x</value><field name='b'/></field></fields></xfdf>",
            "<xfdf xmlns='http://ns.adobe.com/xfdf/'><fields><field name='a'><value><b>rich text</b></value></field></fields></xfdf>",
            "<xfdf xmlns='http://ns.adobe.com/xfdf/'><fields/><annots/></xfdf>",
            "<?script run='no'?><xfdf xmlns='http://ns.adobe.com/xfdf/'><fields/></xfdf>",
            "<xfdf><fields/></xfdf>",
        ] { assert!(parse_values(invalid.as_bytes(), "xfdf").is_err(), "accepted {invalid}"); }
        let deep = format!(
            "<xfdf xmlns='{XFDF_NS}'><fields>{}{}{}</fields></xfdf>",
            "<field name='a'>".repeat(40),
            "<value>x</value>",
            "</field>".repeat(40)
        );
        assert!(parse_values(deep.as_bytes(), "xfdf").is_err());
        assert!(parse_values(&vec![b' '; IMPORT_LIMIT + 1], "json").is_err());
        assert!(strict_plan_from_json(&info, &json!({"nom":"valid", "pays":"unknown"})).is_err());
        assert!(
            strict_plan_from_json(&info, &json!({"nom":"valid", "missing":"unknown"})).is_err()
        );
    }

    fn fdf(values: lopdf::Dictionary) -> Vec<u8> {
        let mut document = lopdf::Document::with_version("1.2");
        document.reference_table.cross_reference_type = lopdf::xref::XrefType::CrossReferenceTable;
        let root = document.add_object(lopdf::dictionary! { "FDF"=>values });
        document.trailer.set("Root", root);
        let mut bytes = Vec::new();
        document.save_to(&mut bytes).unwrap();
        bytes[1..4].copy_from_slice(b"FDF");
        bytes
    }

    #[test]
    fn bounded_fdf_value_subset_decodes_unicode_and_refuses_actions_and_cycles() {
        use lopdf::{dictionary, Object};
        let bytes = fdf(
            dictionary! { "F"=>Object::string_literal("C:/not-followed.pdf"), "Fields"=>vec![
                Object::Dictionary(dictionary! { "T"=>Object::string_literal("nom"), "V"=>lopdf::text_string("Élodie 日本") }),
                Object::Dictionary(dictionary! { "T"=>Object::string_literal("ok"), "V"=>Object::Name(b"Yes".to_vec()) }),
                Object::Dictionary(dictionary! { "T"=>Object::string_literal("langues"), "V"=>vec![Object::string_literal("FR"),Object::string_literal("BE")] }),
                Object::Dictionary(dictionary! { "T"=>Object::string_literal("person"), "Kids"=>vec![Object::Dictionary(dictionary! { "T"=>Object::string_literal("name"), "V"=>Object::string_literal("Test") })] }),
            ] },
        );
        assert_eq!(
            parse_values(&bytes, "fdf").unwrap(),
            json!({"nom":"Élodie 日本", "ok":"Yes", "langues":["FR","BE"], "person.name":"Test"})
        );
        let invalid = fdf(
            dictionary! { "Fields"=>vec![Object::Dictionary(dictionary! { "T"=>Object::string_literal("nom"), "V"=>Object::string_literal("x"), "AA"=>dictionary! { "S"=>"JavaScript", "JS"=>Object::string_literal("throw 1") } })] },
        );
        assert!(parse_values(&invalid, "fdf").is_err());
        let duplicate = fdf(dictionary! { "Fields"=>vec![
        Object::Dictionary(dictionary! { "T"=>Object::string_literal("x"), "V"=>Object::string_literal("a") }),
        Object::Dictionary(dictionary! { "T"=>Object::string_literal("x"), "V"=>Object::string_literal("b") })] });
        assert!(parse_values(&duplicate, "fdf").is_err());
        let mut cyclic = lopdf::Document::with_version("1.2");
        cyclic.reference_table.cross_reference_type = lopdf::xref::XrefType::CrossReferenceTable;
        let field = cyclic.new_object_id();
        cyclic.objects.insert(field, Object::Dictionary(dictionary! { "T"=>Object::string_literal("x"), "Kids"=>vec![Object::Reference(field)] }));
        let root = cyclic.add_object(
            dictionary! { "FDF"=>dictionary! { "Fields"=>vec![Object::Reference(field)] } },
        );
        cyclic.trailer.set("Root", root);
        let mut bytes = Vec::new();
        cyclic.save_to(&mut bytes).unwrap();
        bytes[1..4].copy_from_slice(b"FDF");
        assert!(parse_values(&bytes, "fdf").is_err());
    }
}
