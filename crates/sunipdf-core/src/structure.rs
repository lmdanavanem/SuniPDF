//! Catalog and page relationships travel with selected pages; omitted pages are never imported.

use crate::{
    bookmarks::{Node, Tree},
    EngineError,
};
use lopdf::{Dictionary, Object, ObjectId};
use std::collections::{BTreeMap, HashMap, HashSet};

const MAX_NODES: usize = 250_000;
const MAX_NAMES: usize = 4096;
fn invalid(reason: &str) -> EngineError {
    EngineError::Other(format!("Cannot preserve PDF structure: {reason}"))
}
pub(crate) struct Source {
    pdf: lopdf::Document,
    pages: Vec<ObjectId>,
}
impl Source {
    pub(crate) fn from_bytes(bytes: &[u8], password: Option<&str>) -> Result<Self, EngineError> {
        if bytes.len() > 128 * 1024 * 1024 {
            return Err(invalid("input exceeds 128 MiB"));
        }
        let pdf = crate::security::load(bytes, password)?;
        if pdf.objects.len() > MAX_NODES {
            return Err(invalid("input object limit"));
        }
        if pdf
            .encryption_state
            .as_ref()
            .is_some_and(|state| state.permissions().bits() & (1 << 4) == 0)
        {
            return Err(EngineError::SecurityRestricted);
        }
        let pages = pdf.get_pages().into_values().collect::<Vec<_>>();
        if pages.len() > 65_535 {
            return Err(invalid("input page limit"));
        }
        Tree::read(&pdf)?;
        Ok(Self { pdf, pages })
    }
}
fn dict<'a>(pdf: &'a lopdf::Document, object: &'a Object) -> Result<&'a Dictionary, EngineError> {
    pdf.dereference(object)
        .map_err(crate::security::lopdf_err)?
        .1
        .as_dict()
        .map_err(crate::security::lopdf_err)
}
fn array<'a>(pdf: &'a lopdf::Document, object: &'a Object) -> Result<&'a Vec<Object>, EngineError> {
    pdf.dereference(object)
        .map_err(crate::security::lopdf_err)?
        .1
        .as_array()
        .map_err(crate::security::lopdf_err)
}
fn names(pdf: &lopdf::Document, root: &Object) -> Result<BTreeMap<Vec<u8>, Object>, EngineError> {
    fn visit(
        pdf: &lopdf::Document,
        node: &Object,
        depth: usize,
        seen: &mut HashSet<ObjectId>,
        out: &mut BTreeMap<Vec<u8>, Object>,
    ) -> Result<(), EngineError> {
        if depth > 32 || out.len() > MAX_NAMES {
            return Err(invalid("name tree exceeds its limits"));
        }
        if let Object::Reference(id) = node {
            if !seen.insert(*id) {
                return Err(invalid("cyclic name tree"));
            }
        }
        let node = dict(pdf, node)?;
        match (node.get(b"Names").ok(), node.get(b"Kids").ok()) {
            (Some(entries), None) => {
                let entries = array(pdf, entries)?;
                if entries.len() % 2 != 0 || entries.len() > MAX_NAMES * 2 {
                    return Err(invalid("malformed name tree entries"));
                }
                let mut previous: Option<Vec<u8>> = None;
                for pair in entries.chunks_exact(2) {
                    let key = pair[0]
                        .as_str()
                        .map_err(crate::security::lopdf_err)?
                        .to_vec();
                    if key.len() > 1024
                        || previous.as_ref().is_some_and(|previous| previous >= &key)
                        || out.insert(key.clone(), pair[1].clone()).is_some()
                    {
                        return Err(invalid("duplicate, oversized or unsorted name tree key"));
                    }
                    previous = Some(key);
                }
            }
            (None, Some(kids)) => {
                let kids = array(pdf, kids)?;
                if kids.len() > MAX_NAMES {
                    return Err(invalid("name tree fan-out limit"));
                }
                for child in kids {
                    visit(pdf, child, depth + 1, seen, out)?;
                }
            }
            _ => return Err(invalid("name tree must contain Names or Kids")),
        }
        if out.len() > MAX_NAMES {
            return Err(invalid("name tree entry limit"));
        }
        Ok(())
    }
    let mut result = BTreeMap::new();
    visit(pdf, root, 0, &mut HashSet::new(), &mut result)?;
    Ok(result)
}
pub(crate) fn destinations(
    pdf: &lopdf::Document,
) -> Result<BTreeMap<Vec<u8>, Object>, EngineError> {
    let catalog = pdf.catalog().map_err(crate::security::lopdf_err)?;
    let mut result = BTreeMap::new();
    if let Ok(old) = catalog.get(b"Dests") {
        for (key, value) in dict(pdf, old)? {
            if result.insert(key.clone(), value.clone()).is_some() {
                return Err(invalid("duplicate named destination"));
            }
        }
    }
    if let Ok(all) = catalog.get(b"Names") {
        if let Ok(tree) = dict(pdf, all)?.get(b"Dests") {
            for (key, value) in names(pdf, tree)? {
                if result.insert(key, value).is_some() {
                    return Err(invalid("duplicate named destination"));
                }
            }
        }
    }
    if result.len() > MAX_NAMES {
        return Err(invalid("named destination limit"));
    }
    Ok(result)
}
pub(crate) fn destination(
    pdf: &lopdf::Document,
    value: &Object,
    named: &BTreeMap<Vec<u8>, Object>,
    selected: &HashMap<ObjectId, ObjectId>,
) -> Result<Option<Object>, EngineError> {
    let mut current = value;
    let mut visited = HashSet::new();
    for _ in 0..32 {
        let resolved = pdf
            .dereference(current)
            .map_err(crate::security::lopdf_err)?
            .1;
        match resolved {
            Object::Name(name) | Object::String(name, _) => {
                if !visited.insert(name.clone()) {
                    return Err(invalid("named destination cycle"));
                }
                current = named
                    .get(name)
                    .ok_or_else(|| invalid("missing named destination"))?;
            }
            Object::Dictionary(dict) => {
                current = dict.get(b"D").map_err(crate::security::lopdf_err)?;
            }
            Object::Array(values) => {
                if values.len() < 2 || values.len() > 6 {
                    return Err(invalid("invalid destination array"));
                }
                let page = values[0]
                    .as_reference()
                    .map_err(|_| invalid("local destinations require page references"))?;
                if !selected.contains_key(&page) {
                    return Ok(None);
                }
                let mode = values[1].as_name().map_err(crate::security::lopdf_err)?;
                let length = match mode {
                    b"XYZ" => 5,
                    b"Fit" | b"FitB" => 2,
                    b"FitH" | b"FitV" | b"FitBH" | b"FitBV" => 3,
                    b"FitR" => 6,
                    _ => return Err(invalid("unknown local destination mode")),
                };
                if values.len() != length
                    || values.iter().skip(2).any(|v| {
                        !matches!(v, Object::Null | Object::Integer(_) | Object::Real(_))
                            || matches!(v,Object::Real(n) if !n.is_finite())
                    })
                {
                    return Err(invalid("invalid local destination coordinates"));
                }
                return Ok(Some(Object::Array(values.clone())));
            }
            _ => return Err(invalid("unsupported local destination")),
        }
    }
    Err(invalid("destination reference depth"))
}
fn sanitize_destinations(
    pdf: &mut lopdf::Document,
    original: &lopdf::Document,
    named: &BTreeMap<Vec<u8>, Object>,
    selected: &HashMap<ObjectId, ObjectId>,
) -> Result<(), EngineError> {
    fn edit(
        value: &mut Object,
        original: &lopdf::Document,
        named: &BTreeMap<Vec<u8>, Object>,
        selected: &HashMap<ObjectId, ObjectId>,
        depth: usize,
    ) -> Result<(), EngineError> {
        if depth > 64 {
            return Err(invalid("inline object depth limit"));
        }
        let dictionary = match value {
            Object::Dictionary(dict) => Some(dict),
            Object::Stream(stream) => Some(&mut stream.dict),
            Object::Array(values) => {
                for value in values {
                    edit(value, original, named, selected, depth + 1)?;
                }
                None
            }
            _ => None,
        };
        if let Some(dictionary) = dictionary {
            if let Ok(action) = dictionary.get(b"A").cloned() {
                if let Ok(action) = dict(original, &action) {
                    if action.get(b"S").ok().and_then(|value| value.as_name().ok()) == Some(b"GoTo")
                        && destination(
                            original,
                            action.get(b"D").map_err(crate::security::lopdf_err)?,
                            named,
                            selected,
                        )?
                        .is_none()
                    {
                        dictionary.remove(b"A");
                    }
                }
            }
            if let Ok(value) = dictionary.get(b"Dest").cloned() {
                match destination(original, &value, named, selected)? {
                    Some(value) => dictionary.set("Dest", value),
                    None => {
                        dictionary.remove(b"Dest");
                    }
                }
            }
            if dictionary.get(b"S").ok().and_then(|v| v.as_name().ok()) == Some(b"GoTo") {
                let value = dictionary
                    .get(b"D")
                    .map_err(crate::security::lopdf_err)?
                    .clone();
                match destination(original, &value, named, selected)? {
                    Some(value) => dictionary.set("D", value),
                    None => {
                        dictionary.remove(b"S");
                        dictionary.remove(b"D");
                    }
                }
            }
            for (_, value) in dictionary.iter_mut() {
                edit(value, original, named, selected, depth + 1)?;
            }
        }
        Ok(())
    }
    for value in pdf.objects.values_mut() {
        edit(value, original, named, selected, 0)?;
    }
    Ok(())
}
fn page_dictionary(pdf: &lopdf::Document, page: ObjectId) -> Result<Dictionary, EngineError> {
    let mut result = pdf
        .get_dictionary(page)
        .map_err(crate::security::lopdf_err)?
        .clone();
    let mut parent = result
        .get(b"Parent")
        .ok()
        .and_then(|v| v.as_reference().ok());
    let mut seen = HashSet::new();
    for _ in 0..32 {
        let Some(id) = parent else {
            result.remove(b"Parent");
            return Ok(result);
        };
        if !seen.insert(id) {
            return Err(invalid("page inheritance cycle"));
        }
        let ancestor = pdf.get_dictionary(id).map_err(crate::security::lopdf_err)?;
        for key in [b"Resources".as_slice(), b"MediaBox", b"CropBox", b"Rotate"] {
            if !result.has(key) {
                if let Ok(value) = ancestor.get(key) {
                    result.set(key, value.clone());
                }
            }
        }
        parent = ancestor
            .get(b"Parent")
            .ok()
            .map(|v| v.as_reference().map_err(crate::security::lopdf_err))
            .transpose()?;
    }
    Err(invalid("page inheritance depth limit"))
}
fn field_roots(
    pdf: &mut lopdf::Document,
    original: &lopdf::Document,
    selected_widgets: &HashSet<ObjectId>,
) -> Result<Vec<Object>, EngineError> {
    fn retain(
        pdf: &mut lopdf::Document,
        original: &lopdf::Document,
        value: &Object,
        widgets: &HashSet<ObjectId>,
        seen: &mut HashSet<ObjectId>,
        depth: usize,
    ) -> Result<bool, EngineError> {
        if depth > 32 || seen.len() > MAX_NAMES {
            return Err(invalid("form field tree exceeds limits"));
        }
        let id = value
            .as_reference()
            .map_err(|_| invalid("form fields must be indirect"))?;
        if !seen.insert(id) {
            return Err(invalid("cyclic or shared form field tree"));
        }
        let mut field = pdf
            .get_dictionary(id)
            .map_err(crate::security::lopdf_err)?
            .clone();
        let mut kids = Vec::new();
        if let Ok(children) = field.get(b"Kids") {
            for child in array(original, children)? {
                if retain(pdf, original, child, widgets, seen, depth + 1)? {
                    kids.push(child.clone());
                }
            }
            field.set("Kids", kids.clone());
        }
        let keep = widgets.contains(&id) || !kids.is_empty();
        if keep {
            pdf.objects.insert(id, Object::Dictionary(field));
        } else {
            pdf.objects.remove(&id);
        }
        Ok(keep)
    }
    let catalog = original.catalog().map_err(crate::security::lopdf_err)?;
    let Some(form) = catalog.get(b"AcroForm").ok() else {
        return Ok(Vec::new());
    };
    let form = dict(original, form)?;
    if form.has(b"XFA") {
        return Err(invalid("XFA preservation requires a dedicated converter"));
    }
    let fields = array(
        original,
        form.get(b"Fields").map_err(crate::security::lopdf_err)?,
    )?;
    let mut roots = Vec::new();
    let mut seen = HashSet::new();
    for field in fields {
        if retain(pdf, original, field, selected_widgets, &mut seen, 0)? {
            roots.push(field.clone());
        }
    }
    for id in selected_widgets {
        if !seen.contains(id) {
            return Err(invalid("widget is missing from AcroForm field tree"));
        }
    }
    Ok(roots)
}
fn unique_name(
    names: &mut HashSet<Vec<u8>>,
    name: &[u8],
    source: usize,
) -> Result<Vec<u8>, EngineError> {
    if names.insert(name.to_vec()) {
        return Ok(name.to_vec());
    }
    let mut renamed = format!("S{}:", source + 1).into_bytes();
    renamed.extend_from_slice(name);
    if renamed.len() > 1024 || !names.insert(renamed.clone()) {
        return Err(invalid("cannot disambiguate merged names"));
    }
    Ok(renamed)
}
fn copy(
    source: &lopdf::Document,
    target: &mut lopdf::Document,
    map: &HashMap<ObjectId, ObjectId>,
    copied: &mut HashMap<ObjectId, ObjectId>,
    value: &Object,
) -> Result<Object, EngineError> {
    let copied = crate::forms::deep_copy(source, target, map, copied, 0, value, 64)?;
    if target.objects.len() > 500_000 {
        return Err(invalid("output object limit"));
    }
    Ok(copied)
}

fn form_resources(
    source: &lopdf::Document,
    target: &mut lopdf::Document,
    map: &HashMap<ObjectId, ObjectId>,
    copied: &mut HashMap<ObjectId, ObjectId>,
    form: &Dictionary,
    merged: &mut Dictionary,
    index: usize,
) -> Result<HashMap<Vec<u8>, Vec<u8>>, EngineError> {
    let mut aliases = HashMap::new();
    let Some(resources) = form.get(b"DR").ok() else {
        return Ok(aliases);
    };
    let resources = copy(source, target, map, copied, resources)?
        .as_dict()
        .map_err(crate::security::lopdf_err)?
        .clone();
    for (kind, entries) in resources {
        let entries = dict(target, &entries)?.clone();
        let mut existing = merged
            .get(&kind)
            .ok()
            .map(|value| dict(target, value).cloned())
            .transpose()?
            .unwrap_or_default();
        for (key, value) in entries {
            let mut name = key.clone();
            if let Ok(previous) = existing.get(&key) {
                let equal = target
                    .dereference(previous)
                    .map_err(crate::security::lopdf_err)?
                    .1
                    == target
                        .dereference(&value)
                        .map_err(crate::security::lopdf_err)?
                        .1;
                if equal {
                    continue;
                }
                if kind != b"Font" {
                    return Err(invalid(
                        "conflicting non-font form resources require explicit normalization",
                    ));
                }
                let mut found = false;
                for counter in 1..=MAX_NAMES {
                    name = format!("S{}_{}_", index + 1, counter).into_bytes();
                    name.extend_from_slice(&key);
                    if !existing.has(&name) {
                        found = true;
                        break;
                    }
                }
                if !found {
                    return Err(invalid("form font resource collision limit"));
                }
                aliases.insert(key, name.clone());
            }
            existing.set(name, value);
        }
        merged.set(kind, existing);
    }
    Ok(aliases)
}
fn field_appearances(
    pdf: &mut lopdf::Document,
    roots: &[Object],
    form: &Dictionary,
    aliases: &HashMap<Vec<u8>, Vec<u8>>,
) -> Result<HashSet<ObjectId>, EngineError> {
    let mut pending = roots.to_vec();
    let mut seen = HashSet::new();
    while let Some(value) = pending.pop() {
        let id = value.as_reference().map_err(crate::security::lopdf_err)?;
        if !seen.insert(id) || seen.len() > MAX_NAMES {
            return Err(invalid("field appearance tree limit"));
        }
        let mut field = pdf
            .get_dictionary(id)
            .map_err(crate::security::lopdf_err)?
            .clone();
        if roots.contains(&value) {
            for key in [b"DA".as_slice(), b"Q"] {
                if !field.has(key) {
                    if let Ok(value) = form.get(key) {
                        field.set(key, value.clone());
                    }
                }
            }
        }
        if let Ok(appearance) = field.get(b"DA") {
            let bytes = appearance.as_str().map_err(crate::security::lopdf_err)?;
            if bytes.len() > 32768 {
                return Err(invalid("field default appearance size limit"));
            }
            let mut content = lopdf::content::Content::decode_strict(bytes)
                .map_err(crate::security::lopdf_err)?;
            if content.operations.len() > 256 {
                return Err(invalid("field default appearance operation limit"));
            }
            for operation in &mut content.operations {
                if operation.operator == "Tf" {
                    let Some(Object::Name(font)) = operation.operands.first_mut() else {
                        return Err(invalid("malformed field default font"));
                    };
                    if let Some(alias) = aliases.get(font) {
                        *font = alias.clone();
                    }
                }
            }
            field.set(
                "DA",
                Object::string_literal(content.encode().map_err(crate::security::lopdf_err)?),
            );
        }
        if let Ok(kids) = field.get(b"Kids") {
            pending.extend(array(pdf, kids)?.iter().cloned());
        }
        pdf.objects.insert(id, Object::Dictionary(field));
    }
    Ok(seen)
}

pub(crate) fn preserve(
    target: &mut lopdf::Document,
    sources: &[&Source],
    provenance: &[Option<(usize, usize)>],
) -> Result<(), EngineError> {
    let pages: Vec<_> = target.get_pages().into_values().collect();
    if pages.len() != provenance.len() || sources.len() > 128 {
        return Err(invalid("page mapping mismatch"));
    }
    let mut maps = vec![HashMap::new(); sources.len()];
    for (target_page, source) in pages.iter().zip(provenance) {
        if let Some((source, page)) = *source {
            let source_page = sources
                .get(source)
                .and_then(|source| source.pages.get(page))
                .ok_or_else(|| invalid("invalid provenance"))?;
            maps[source].entry(*source_page).or_insert(*target_page);
        }
    }
    let mut outlines = Tree::default();
    let mut output_names: BTreeMap<Vec<u8>, BTreeMap<Vec<u8>, Object>> = BTreeMap::new();
    let mut name_keys: HashMap<Vec<u8>, HashSet<Vec<u8>>> = HashMap::new();
    let mut fields = Vec::new();
    let mut field_names = HashSet::new();
    let mut merged_form_resources = Dictionary::new();
    let mut calculation_order = Vec::new();
    let mut root_associated = Vec::new();
    let mut ocgs = Vec::new();
    let mut oc_order = Vec::new();
    let mut oc_on = Vec::new();
    let mut oc_off = Vec::new();
    let mut need_appearances = false;
    let mut sig_flags = 0i64;
    for (source_index, source) in sources.iter().enumerate() {
        let original = &source.pdf;
        let mut sanitized = original.clone();
        let selected = &maps[source_index];
        let named = destinations(original)?;
        sanitize_destinations(&mut sanitized, original, &named, selected)?;
        let mut widgets = HashSet::new();
        let mut omitted_annotations = HashSet::new();
        for page in &source.pages {
            let page_dict = original
                .get_dictionary(*page)
                .map_err(crate::security::lopdf_err)?;
            if let Ok(annots) = page_dict.get(b"Annots") {
                for annotation in array(original, annots)? {
                    let id = annotation
                        .as_reference()
                        .map_err(|_| invalid("annotations must be indirect"))?;
                    if selected.contains_key(page) {
                        if dict(original, annotation)?
                            .get(b"Subtype")
                            .ok()
                            .and_then(|v| v.as_name().ok())
                            == Some(b"Widget")
                        {
                            widgets.insert(id);
                        }
                    } else {
                        omitted_annotations.insert(id);
                    }
                }
            }
            if !selected.contains_key(page) {
                sanitized.objects.remove(page);
            }
        }
        let roots = field_roots(&mut sanitized, original, &widgets)?;
        for id in omitted_annotations {
            sanitized.objects.remove(&id);
        }
        let page_dicts = selected
            .keys()
            .map(|page| Ok((*page, page_dictionary(&sanitized, *page)?)))
            .collect::<Result<HashMap<_, _>, EngineError>>()?;
        // Unselected page-tree nodes cannot be reached through a copied resource/action graph.
        let page_trees: Vec<_> = sanitized
            .objects
            .iter()
            .filter_map(|(id, value)| {
                value
                    .as_dict()
                    .ok()
                    .filter(|d| {
                        d.get(b"Type").ok().and_then(|v| v.as_name().ok()) == Some(b"Pages")
                    })
                    .map(|_| *id)
            })
            .collect();
        for id in page_trees {
            sanitized.objects.remove(&id);
        }
        let mut copied = HashMap::new();
        let catalog = original.catalog().map_err(crate::security::lopdf_err)?;
        let mut kept_fields = HashSet::new();
        if let Ok(form) = catalog.get(b"AcroForm") {
            let form = dict(original, form)?;
            let aliases = form_resources(
                &sanitized,
                target,
                selected,
                &mut copied,
                form,
                &mut merged_form_resources,
                source_index,
            )?;
            kept_fields = field_appearances(&mut sanitized, &roots, form, &aliases)?;
        }
        for (target_page, provenance) in pages.iter().zip(provenance) {
            let Some((index, page_index)) = *provenance else {
                continue;
            };
            if index != source_index {
                continue;
            }
            let source_page = source.pages[page_index];
            let mut page = page_dicts[&source_page].clone();
            if let Ok(annotations) = page.get(b"Annots") {
                for annotation in array(&sanitized, annotations)? {
                    let id = annotation
                        .as_reference()
                        .map_err(crate::security::lopdf_err)?;
                    if selected[&source_page] != *target_page && widgets.contains(&id) {
                        return Err(invalid(
                            "duplicated form widgets require explicit field duplication",
                        ));
                    }
                    copied.remove(&id);
                }
            }
            let parent = target
                .get_dictionary(*target_page)
                .map_err(crate::security::lopdf_err)?
                .get(b"Parent")
                .map_err(crate::security::lopdf_err)?
                .clone();
            page.remove(b"Parent");
            let mut page = copy(
                &sanitized,
                target,
                selected,
                &mut copied,
                &Object::Dictionary(page),
            )?
            .as_dict()
            .map_err(crate::security::lopdf_err)?
            .clone();
            page.set("Parent", parent);
            if let Ok(annotations) = page.get(b"Annots") {
                let annotations = array(target, annotations)?.clone();
                for annotation in annotations {
                    let id = annotation
                        .as_reference()
                        .map_err(crate::security::lopdf_err)?;
                    target
                        .get_dictionary_mut(id)
                        .map_err(crate::security::lopdf_err)?
                        .set("P", *target_page);
                }
            }
            target
                .objects
                .insert(*target_page, Object::Dictionary(page));
        }
        let tree = Tree::read(original)?;
        let offset = outlines.nodes.len();
        for node in tree.nodes {
            let mut metadata = node.dict;
            for key in [b"Parent".as_slice(), b"First", b"Last", b"Prev", b"Next"] {
                metadata.remove(key);
            }
            if let Ok(value) = metadata.get(b"Dest").cloned() {
                match destination(original, &value, &named, selected)? {
                    Some(value) => metadata.set("Dest", value),
                    None => {
                        metadata.remove(b"Dest");
                    }
                }
            }
            if let Ok(value) = metadata.get(b"A").cloned() {
                if dict(original, &value)?
                    .get(b"S")
                    .ok()
                    .and_then(|v| v.as_name().ok())
                    == Some(b"GoTo")
                {
                    let action = dict(original, &value)?;
                    if let Some(destination) = destination(
                        original,
                        action.get(b"D").map_err(crate::security::lopdf_err)?,
                        &named,
                        selected,
                    )? {
                        let mut action = action.clone();
                        action.set("D", destination);
                        metadata.set("A", action);
                    } else {
                        metadata.remove(b"A");
                    }
                }
            }
            let metadata = copy(
                &sanitized,
                target,
                selected,
                &mut copied,
                &Object::Dictionary(metadata),
            )?
            .as_dict()
            .map_err(crate::security::lopdf_err)?
            .clone();
            outlines.nodes.push(Node {
                dict: metadata,
                children: node.children.into_iter().map(|i| i + offset).collect(),
                parent: node.parent.map(|i| i + offset),
            });
        }
        outlines
            .roots
            .extend(tree.roots.into_iter().map(|i| i + offset));
        if outlines.nodes.len() > crate::bookmarks::MAX_BOOKMARKS {
            return Err(invalid("merged outline limit"));
        }
        let catalog = original.catalog().map_err(crate::security::lopdf_err)?;
        let names_dict = catalog
            .get(b"Names")
            .ok()
            .map(|value| dict(original, value))
            .transpose()?;
        let mut groups = BTreeMap::new();
        let mut kept_destinations = BTreeMap::new();
        for (key, value) in &named {
            if let Some(value) = destination(original, value, &named, selected)? {
                kept_destinations.insert(key.clone(), value);
            }
        }
        groups.insert(b"Dests".to_vec(), kept_destinations);
        if let Some(names) = names_dict {
            for (kind, root) in names {
                if kind != b"Dests" {
                    groups.insert(kind.clone(), self::names(original, root)?);
                }
            }
        }
        for (kind, entries) in groups {
            for (key, value) in entries {
                let key = unique_name(
                    name_keys.entry(kind.clone()).or_default(),
                    &key,
                    source_index,
                )?;
                let value = copy(&sanitized, target, selected, &mut copied, &value)?;
                output_names
                    .entry(kind.clone())
                    .or_default()
                    .insert(key, value);
            }
        }
        if let Ok(associated) = catalog.get(b"AF") {
            for value in array(original, associated)? {
                root_associated.push(copy(&sanitized, target, selected, &mut copied, value)?);
            }
        }
        if let Ok(form) = catalog.get(b"AcroForm") {
            let form = dict(original, form)?;
            need_appearances |= form
                .get(b"NeedAppearances")
                .ok()
                .and_then(|v| v.as_bool().ok())
                .unwrap_or(false);
            sig_flags |= form
                .get(b"SigFlags")
                .ok()
                .and_then(|v| v.as_i64().ok())
                .unwrap_or(0);
            if let Ok(order) = form.get(b"CO") {
                for value in array(original, order)? {
                    if value
                        .as_reference()
                        .is_ok_and(|id| kept_fields.contains(&id))
                    {
                        calculation_order.push(copy(
                            &sanitized,
                            target,
                            selected,
                            &mut copied,
                            value,
                        )?);
                    }
                }
            }
            for root in roots {
                let id = root.as_reference().map_err(crate::security::lopdf_err)?;
                let mut field = sanitized
                    .get_dictionary(id)
                    .map_err(crate::security::lopdf_err)?
                    .clone();
                if let Ok(name) = field.get(b"T") {
                    let name =
                        lopdf::decode_text_string(name).map_err(crate::security::lopdf_err)?;
                    let renamed = unique_name(&mut field_names, name.as_bytes(), source_index)?;
                    if renamed != name.as_bytes() {
                        if form.has(b"CO")
                            || sanitized.objects.values().any(|object| {
                                object.as_dict().is_ok_and(|d| {
                                    d.has(b"AA")
                                        || d.get(b"S").ok().and_then(|v| v.as_name().ok())
                                            == Some(b"JavaScript")
                                })
                            })
                        {
                            return Err(invalid("colliding field names with calculations/actions require explicit normalization"));
                        }
                        field.set(
                            "T",
                            lopdf::text_string(
                                &String::from_utf8(renamed)
                                    .map_err(|_| invalid("field name encoding"))?,
                            ),
                        );
                    }
                }
                for key in [b"DA".as_slice(), b"Q"] {
                    if !field.has(key) {
                        if let Ok(value) = form.get(key) {
                            field.set(key, value.clone());
                        }
                    }
                }
                sanitized
                    .objects
                    .insert(id, Object::Dictionary(field.clone()));
                let value = if let Some(target_id) = copied.get(&id).copied() {
                    let field = copy(
                        &sanitized,
                        target,
                        selected,
                        &mut copied,
                        &Object::Dictionary(field),
                    )?;
                    target.objects.insert(target_id, field);
                    Object::Reference(target_id)
                } else {
                    copy(&sanitized, target, selected, &mut copied, &root)?
                };
                fields.push(value);
            }
        }
        if let Ok(properties) = catalog.get(b"OCProperties") {
            let properties = dict(original, properties)?;
            for value in array(
                original,
                properties
                    .get(b"OCGs")
                    .map_err(crate::security::lopdf_err)?,
            )? {
                ocgs.push(copy(&sanitized, target, selected, &mut copied, value)?);
            }
            if let Ok(default) = properties.get(b"D") {
                let default = dict(original, default)?;
                if default.has(b"AS")
                    || default.has(b"RBGroups")
                    || default
                        .get(b"BaseState")
                        .ok()
                        .and_then(|v| v.as_name().ok())
                        .is_some_and(|name| name != b"ON")
                {
                    return Err(invalid(
                        "complex optional-content configurations require explicit normalization",
                    ));
                }
                for (key, output) in [
                    (b"Order".as_slice(), &mut oc_order),
                    (b"ON", &mut oc_on),
                    (b"OFF", &mut oc_off),
                ] {
                    if let Ok(values) = default.get(key) {
                        for value in array(original, values)? {
                            output.push(copy(&sanitized, target, selected, &mut copied, value)?);
                        }
                    }
                }
            }
        }
    }
    outlines.write(target)?;
    let catalog = target.catalog_mut().map_err(crate::security::lopdf_err)?;
    catalog.remove(b"Dests");
    catalog.remove(b"Names");
    catalog.remove(b"AcroForm");
    catalog.remove(b"OCProperties");
    catalog.remove(b"AF");
    let mut names = Dictionary::new();
    for (kind, entries) in output_names {
        if !entries.is_empty() {
            let values: Vec<_> = entries
                .into_iter()
                .flat_map(|(key, value)| [Object::String(key, lopdf::StringFormat::Literal), value])
                .collect();
            let mut tree = Dictionary::new();
            tree.set("Names", values);
            names.set(kind, tree);
        }
    }
    if !names.is_empty() {
        catalog.set("Names", names);
    }
    if !fields.is_empty() {
        let mut form = Dictionary::new();
        form.set("Fields", fields);
        form.set("NeedAppearances", need_appearances);
        form.set("SigFlags", sig_flags);
        if !merged_form_resources.is_empty() {
            form.set("DR", merged_form_resources);
        }
        if !calculation_order.is_empty() {
            form.set("CO", calculation_order);
        }
        catalog.set("AcroForm", form);
    }
    if !root_associated.is_empty() {
        catalog.set("AF", root_associated);
    }
    if !ocgs.is_empty() {
        let mut default = Dictionary::new();
        default.set("BaseState", "ON");
        default.set("Order", oc_order);
        default.set("ON", oc_on);
        default.set("OFF", oc_off);
        let mut properties = Dictionary::new();
        properties.set("OCGs", ocgs);
        properties.set("D", default);
        catalog.set("OCProperties", properties);
    }
    target.prune_objects();
    let output_pages: HashSet<_> = pages.into_iter().collect();
    if target.objects.iter().any(|(id, object)| {
        object.as_dict().is_ok_and(|dictionary| {
            dictionary
                .get(b"Type")
                .ok()
                .and_then(|value| value.as_name().ok())
                == Some(b"Page")
        }) && !output_pages.contains(id)
    }) {
        return Err(invalid(
            "an extension references a page outside the selected output tree",
        ));
    }
    if target
        .objects
        .values()
        .filter_map(|object| object.as_stream().ok())
        .try_fold(0usize, |sum, stream| sum.checked_add(stream.content.len()))
        .is_none_or(|size| size > 512 * 1024 * 1024)
    {
        return Err(invalid("output stream size limit"));
    }
    Ok(())
}
