//! Value-only FDF exchange, with partial field names and native button states.

use crate::forms::{FieldKind, FieldValue, FormField, FormInfo};
use crate::EngineError;
use lopdf::{dictionary, Object};
use std::collections::{BTreeMap, BTreeSet};

fn invalid(message: &str) -> EngineError {
    EngineError::FormValueInvalid(format!("form data: {message}"))
}

#[derive(Default)]
struct Node {
    value: Option<Object>,
    children: BTreeMap<String, Node>,
}

impl Node {
    fn insert(&mut self, name: &str, value: Object) -> Result<(), EngineError> {
        let components: Vec<_> = name.split('.').collect();
        if components.len() > 12
            || components
                .iter()
                .any(|part| part.is_empty() || part.len() > 4096)
            || name.chars().any(char::is_control)
        {
            return Err(invalid("invalid or excessively nested field name"));
        }
        let mut node = self;
        for component in components {
            if node.value.is_some() {
                return Err(invalid("a terminal field cannot also have child fields"));
            }
            node = node.children.entry(component.into()).or_default();
        }
        if node.value.is_some() || !node.children.is_empty() {
            return Err(invalid("duplicate or conflicting field names"));
        }
        node.value = Some(value);
        Ok(())
    }

    fn fields(self, pdf: &mut lopdf::Document) -> Vec<Object> {
        self.children
            .into_iter()
            .map(|(name, node)| {
                let mut field = dictionary! { "T" => lopdf::text_string(&name) };
                if let Some(value) = node.value {
                    field.set("V", value);
                } else {
                    field.set("Kids", node.fields(pdf));
                }
                Object::Reference(pdf.add_object(field))
            })
            .collect()
    }
}

fn button_name(field: &FormField, value: Option<&str>) -> Result<Object, EngineError> {
    let Some(value) = value else {
        return Ok(Object::Name(b"Off".to_vec()));
    };
    let states: BTreeSet<_> = field
        .widgets
        .iter()
        .filter(|widget| {
            field.kind == FieldKind::Checkbox
                || widget
                    .export_value
                    .as_deref()
                    .or(widget.on_state.as_deref())
                    == Some(value)
        })
        .filter_map(|widget| widget.on_state.as_deref())
        .collect();
    if states.len() != 1 {
        return Err(invalid("selected button has no unique appearance state"));
    }
    let state = states.into_iter().next().expect("one state");
    if state.is_empty()
        || state == "Off"
        || !state.is_ascii()
        || state.chars().any(char::is_control)
    {
        return Err(invalid("invalid selected button appearance state"));
    }
    if field.kind == FieldKind::Radio
        && field.widgets.iter().any(|widget| {
            widget.on_state.as_deref() == Some(state)
                && widget
                    .export_value
                    .as_deref()
                    .or(widget.on_state.as_deref())
                    != Some(value)
        })
    {
        return Err(invalid(
            "button appearance state identifies multiple export values",
        ));
    }
    Ok(Object::Name(state.as_bytes().to_vec()))
}

fn value(field: &FormField) -> Result<Object, EngineError> {
    Ok(match (&field.kind, &field.value) {
        (FieldKind::Text, FieldValue::Text(text)) => lopdf::text_string(text),
        (FieldKind::Text, FieldValue::None) => lopdf::text_string(""),
        (FieldKind::Checkbox, FieldValue::Bool(checked)) => {
            button_name(field, checked.then_some(""))?
        }
        (FieldKind::Radio, FieldValue::Choice(values)) if values.len() <= 1 => {
            button_name(field, values.first().map(String::as_str))?
        }
        (FieldKind::Radio | FieldKind::Checkbox, FieldValue::None) => button_name(field, None)?,
        (FieldKind::Combo | FieldKind::List, FieldValue::Choice(values)) => {
            if values.len() > crate::form_data::MAX_CHOICE_VALUES {
                return Err(invalid("choice value count exceeds 4096"));
            }
            if field.kind == FieldKind::List && field.multi_select {
                Object::Array(values.iter().map(|text| lopdf::text_string(text)).collect())
            } else if values.len() <= 1 {
                values
                    .first()
                    .map_or(Object::Null, |text| lopdf::text_string(text))
            } else {
                return Err(invalid("single-choice field has multiple values"));
            }
        }
        (FieldKind::Combo | FieldKind::List, FieldValue::None) => Object::Null,
        _ => return Err(invalid("field value does not match its type")),
    })
}

pub(crate) fn export(info: &FormInfo) -> Result<Vec<u8>, EngineError> {
    let mut root = Node::default();
    let mut count = 0;
    let mut size = 0usize;
    for field in info
        .fields
        .iter()
        .filter(|field| field.kind.fillable() && !field.password)
    {
        count += 1;
        size = size.saturating_add(field.name.len());
        size = size.saturating_add(match &field.value {
            FieldValue::Text(text) => text.len(),
            FieldValue::Choice(values) => values.iter().map(String::len).sum(),
            _ => 0,
        });
        if count > 10_000 || size > crate::form_data::IMPORT_LIMIT {
            return Err(invalid("FDF export exceeds field or data limits"));
        }
        root.insert(&field.name, value(field)?)?;
    }
    let mut pdf = lopdf::Document::with_version("1.2");
    pdf.reference_table.cross_reference_type = lopdf::xref::XrefType::CrossReferenceTable;
    let fields = root.fields(&mut pdf);
    let catalog = pdf.add_object(dictionary! { "FDF" => dictionary! { "Fields" => fields } });
    pdf.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes)?;
    bytes[1..4].copy_from_slice(b"FDF");
    if bytes.len() > crate::form_data::IMPORT_LIMIT {
        return Err(invalid("FDF export exceeds 16 MiB"));
    }
    // Apply the same decoded-value and qualified-name limits as the import boundary.
    crate::form_data::parse_import_values(&bytes, "fdf")?;
    Ok(bytes)
}
