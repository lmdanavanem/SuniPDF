//! Precise local destinations. Names stay byte-exact; viewport nulls are never replaced by zero.
use crate::{
    Destination, DestinationView, Document, EngineError, LinkTarget, PageGeometry, Rotation,
};
use lopdf::{Dictionary, Object, ObjectId};
use std::collections::{BTreeMap, HashMap, HashSet};

fn invalid(reason: &str) -> EngineError {
    EngineError::Other(format!("Invalid local link destination: {reason}"))
}

fn deref<'a>(pdf: &'a lopdf::Document, mut object: &'a Object) -> Result<&'a Object, EngineError> {
    let mut seen = HashSet::new();
    for _ in 0..32 {
        let Object::Reference(id) = object else {
            return Ok(object);
        };
        if !seen.insert(*id) {
            return Err(invalid("reference cycle"));
        }
        object = pdf.get_object(*id).map_err(crate::security::lopdf_err)?;
    }
    Err(invalid("reference depth limit"))
}

fn label(bytes: &[u8]) -> Result<String, EngineError> {
    if bytes.is_empty() || bytes.len() > 1024 {
        return Err(invalid("empty or oversized name"));
    }
    let value = if bytes.starts_with(&[0xfe, 0xff]) || bytes.starts_with(&[0xff, 0xfe]) {
        lopdf::decode_text_string(&Object::string_literal(bytes.to_vec()))
            .map_err(crate::security::lopdf_err)?
    } else {
        String::from_utf8(bytes.to_vec()).unwrap_or_else(|_| {
            lopdf::decode_text_string(&Object::string_literal(bytes.to_vec())).unwrap_or_default()
        })
    };
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(invalid("unsafe destination name"));
    }
    Ok(value)
}

pub(crate) struct Resolver<'a> {
    pdf: &'a lopdf::Document,
    named: BTreeMap<Vec<u8>, Object>,
    names_valid: bool,
    pages: HashMap<ObjectId, usize>,
    identity: HashMap<ObjectId, ObjectId>,
}

impl<'a> Resolver<'a> {
    pub(crate) fn new(pdf: &'a lopdf::Document) -> Self {
        let named = crate::structure::destinations(pdf);
        let pages = pdf
            .get_pages()
            .into_iter()
            .map(|(page, id)| (id, page as usize - 1))
            .collect::<HashMap<_, _>>();
        let identity = pages.keys().map(|id| (*id, *id)).collect();
        Self {
            pdf,
            names_valid: named.is_ok(),
            named: named.unwrap_or_default(),
            pages,
            identity,
        }
    }

    pub(crate) fn page_annotations(
        &self,
        page: usize,
    ) -> Result<Vec<Option<&'a Dictionary>>, EngineError> {
        let id = self
            .pages
            .iter()
            .find_map(|(id, index)| (*index == page).then_some(*id))
            .ok_or(EngineError::PageOutOfRange(page))?;
        let dictionary = self
            .pdf
            .get_dictionary(id)
            .map_err(crate::security::lopdf_err)?;
        let Ok(annotations) = dictionary.get(b"Annots") else {
            return Ok(Vec::new());
        };
        let entries = deref(self.pdf, annotations)?
            .as_array()
            .map_err(crate::security::lopdf_err)?;
        if entries.len() > 65_535 {
            return Err(invalid("annotation count limit"));
        }
        Ok(entries
            .iter()
            .map(|entry| {
                deref(self.pdf, entry)
                    .ok()
                    .and_then(|object| object.as_dict().ok())
            })
            .collect())
    }

    fn named_key(&self, name: &str) -> Result<&Vec<u8>, EngineError> {
        if !self.names_valid {
            return Err(invalid("malformed named destinations"));
        }
        let matches = self
            .named
            .keys()
            .filter(|key| label(key).is_ok_and(|decoded| decoded == name))
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(invalid("missing or ambiguous name"));
        }
        Ok(matches[0])
    }

    pub(crate) fn names(&self) -> Result<Vec<String>, EngineError> {
        if !self.names_valid {
            return Err(invalid("malformed named destinations"));
        }
        let mut labels = self
            .named
            .keys()
            .map(|key| label(key))
            .collect::<Result<Vec<_>, _>>()?;
        labels.sort();
        if labels.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(invalid("ambiguous decoded names"));
        }
        Ok(labels)
    }

    pub(crate) fn fallback_page(&self, annotation: &Dictionary) -> Option<usize> {
        let raw = annotation.get(b"Dest").ok().or_else(|| {
            let action = deref(self.pdf, annotation.get(b"A").ok()?)
                .ok()?
                .as_dict()
                .ok()?;
            if action.get(b"S").and_then(Object::as_name).ok()? != b"GoTo" {
                return None;
            }
            action.get(b"D").ok()
        })?;
        let mut value = raw;
        let mut names = HashSet::new();
        for _ in 0..32 {
            value = deref(self.pdf, value).ok()?;
            match value {
                Object::Array(values) => {
                    // A viewer may retain safe local page navigation when view arguments are
                    // malformed. Precise decoding and all authoring remain strict.
                    return self
                        .pages
                        .get(&values.first()?.as_reference().ok()?)
                        .copied();
                }
                Object::Dictionary(dictionary) => value = dictionary.get(b"D").ok()?,
                Object::Name(bytes) | Object::String(bytes, _)
                    if self.names_valid && bytes.len() <= 1024 && names.insert(bytes.clone()) =>
                {
                    value = self.named.get(bytes)?;
                }
                _ => return None,
            }
        }
        None
    }

    pub(crate) fn destination(
        &self,
        object: &Object,
        geometry: &dyn Fn(usize) -> Result<PageGeometry, EngineError>,
    ) -> Result<Destination, EngineError> {
        let value = crate::structure::destination(self.pdf, object, &self.named, &self.identity)?
            .ok_or_else(|| invalid("target page is absent"))?;
        let array = value.as_array().map_err(crate::security::lopdf_err)?;
        let page_id = array[0]
            .as_reference()
            .map_err(crate::security::lopdf_err)?;
        let page = *self
            .pages
            .get(&page_id)
            .ok_or_else(|| invalid("target page is absent"))?;
        let geo = geometry(page)?;
        let number = |index: usize| -> Result<Option<f32>, EngineError> {
            if matches!(array[index], Object::Null) {
                return Ok(None);
            }
            let value = array[index]
                .as_float()
                .map_err(crate::security::lopdf_err)?;
            if !value.is_finite() {
                return Err(invalid("non-finite coordinate"));
            }
            Ok(Some(value))
        };
        let view = match array[1].as_name().map_err(crate::security::lopdf_err)? {
            b"Fit" => DestinationView::FitPage,
            b"XYZ" => {
                let (x, y) = display_axes(geo, number(2)?, number(3)?);
                if x.is_some_and(|value| !value.is_finite())
                    || y.is_some_and(|value| !value.is_finite())
                {
                    return Err(invalid("coordinate transformation overflow"));
                }
                let zoom = number(4)?;
                if zoom.is_some_and(|zoom| zoom < 0.0) {
                    return Err(invalid("negative zoom"));
                }
                DestinationView::Xyz {
                    x,
                    y,
                    zoom: zoom.filter(|zoom| *zoom > 0.0),
                }
            }
            b"FitH" => {
                let (x, y) = display_axes(geo, None, number(2)?);
                if x.is_some_and(|value| !value.is_finite())
                    || y.is_some_and(|value| !value.is_finite())
                {
                    return Err(invalid("coordinate transformation overflow"));
                }
                DestinationView::FitWidth { x, y }
            }
            _ => return Err(invalid("unsupported precise view")),
        };
        Ok(Destination { page, view })
    }

    pub(crate) fn link(
        &self,
        annotation: &Dictionary,
        geometry: &dyn Fn(usize) -> Result<PageGeometry, EngineError>,
    ) -> Result<(LinkTarget, Option<Destination>), EngineError> {
        let raw = if let Ok(dest) = annotation.get(b"Dest") {
            dest
        } else {
            let action = annotation.get(b"A").map_err(crate::security::lopdf_err)?;
            let action = deref(self.pdf, action)?
                .as_dict()
                .map_err(crate::security::lopdf_err)?;
            match action
                .get(b"S")
                .and_then(Object::as_name)
                .map_err(crate::security::lopdf_err)?
            {
                b"URI" => {
                    let bytes = deref(
                        self.pdf,
                        action.get(b"URI").map_err(crate::security::lopdf_err)?,
                    )?
                    .as_str()
                    .map_err(crate::security::lopdf_err)?;
                    return Ok((
                        LinkTarget::Uri(String::from_utf8_lossy(bytes).into_owned()),
                        None,
                    ));
                }
                b"GoToR" => {
                    if annotation.has(b"AA") {
                        return Err(invalid("additional remote actions"));
                    }
                    return Ok((
                        LinkTarget::RemotePage(crate::plan_links::parse_remote_action(action)?),
                        None,
                    ));
                }
                b"GoTo" => action.get(b"D").map_err(crate::security::lopdf_err)?,
                _ => return Err(invalid("unsupported action")),
            }
        };
        let dest = self.destination(raw, geometry)?;
        let target = match deref(self.pdf, raw)? {
            Object::Name(bytes) | Object::String(bytes, _) => {
                let decoded = label(bytes)?;
                if self.named_key(&decoded)? != bytes {
                    return Err(invalid("ambiguous original name"));
                }
                LinkTarget::Named(decoded)
            }
            _ if dest.view == DestinationView::FitPage => LinkTarget::Page(dest.page),
            _ => LinkTarget::Destination(dest),
        };
        Ok((target, Some(dest)))
    }

    fn named_object(
        &self,
        name: &str,
        current: &Dictionary,
        geometry: &dyn Fn(usize) -> Result<PageGeometry, EngineError>,
    ) -> Result<Object, EngineError> {
        let key = self.named_key(name)?;
        self.destination(&Object::string_literal(key.clone()), geometry)?;
        let original = current.get(b"Dest").ok().or_else(|| {
            current
                .get(b"A")
                .ok()
                .and_then(|action| deref(self.pdf, action).ok())
                .and_then(|action| action.as_dict().ok())
                .and_then(|action| action.get(b"D").ok())
        });
        if let Some(original) = original.and_then(|object| deref(self.pdf, object).ok()) {
            if matches!(original, Object::Name(bytes) | Object::String(bytes, _) if bytes == key) {
                return Ok(original.clone());
            }
        }
        let legacy = self
            .pdf
            .catalog()
            .ok()
            .and_then(|catalog| catalog.get(b"Dests").ok())
            .and_then(|object| deref(self.pdf, object).ok())
            .and_then(|object| object.as_dict().ok())
            .is_some_and(|dictionary| dictionary.has(key));
        Ok(if legacy {
            Object::Name(key.clone())
        } else {
            Object::string_literal(key.clone())
        })
    }
}

pub(crate) fn display_axes(
    geo: PageGeometry,
    x: Option<f32>,
    y: Option<f32>,
) -> (Option<f32>, Option<f32>) {
    let b = geo.bbox;
    match geo.rotation {
        Rotation::R0 => (x.map(|v| v - b.left), y.map(|v| v - b.bottom)),
        Rotation::R90 => (y.map(|v| v - b.bottom), x.map(|v| b.right - v)),
        Rotation::R180 => (x.map(|v| b.right - v), y.map(|v| b.top - v)),
        Rotation::R270 => (y.map(|v| b.top - v), x.map(|v| v - b.left)),
    }
}

fn user_axes(geo: PageGeometry, x: Option<f32>, y: Option<f32>) -> (Option<f32>, Option<f32>) {
    let b = geo.bbox;
    match geo.rotation {
        Rotation::R0 => (x.map(|v| v + b.left), y.map(|v| v + b.bottom)),
        Rotation::R90 => (y.map(|v| b.right - v), x.map(|v| v + b.bottom)),
        Rotation::R180 => (x.map(|v| b.right - v), y.map(|v| b.top - v)),
        Rotation::R270 => (y.map(|v| v + b.left), x.map(|v| b.top - v)),
    }
}

pub(crate) fn authored(
    pdf: &lopdf::Document,
    target: &LinkTarget,
    current: &Dictionary,
    geometry: &dyn Fn(usize) -> Result<PageGeometry, EngineError>,
) -> Result<Object, EngineError> {
    if let LinkTarget::Named(name) = target {
        return Resolver::new(pdf).named_object(name, current, geometry);
    }
    let dest = match target {
        LinkTarget::Page(page) => Destination {
            page: *page,
            view: DestinationView::FitPage,
        },
        LinkTarget::Destination(dest) => *dest,
        _ => return Err(invalid("not a local destination")),
    };
    let id = u32::try_from(dest.page)
        .ok()
        .and_then(|page| page.checked_add(1))
        .and_then(|page| pdf.get_pages().get(&page).copied())
        .ok_or(EngineError::PageOutOfRange(dest.page))?;
    let mut array = vec![Object::Reference(id)];
    let finite = |value: Option<f32>| value.is_none_or(f32::is_finite);
    let number = |value: Option<f32>| value.map_or(Object::Null, Object::Real);
    match dest.view {
        DestinationView::FitPage => array.push(Object::Name(b"Fit".to_vec())),
        DestinationView::Xyz { x, y, zoom } => {
            if !finite(x) || !finite(y) || !finite(zoom) || zoom.is_some_and(|value| value <= 0.0) {
                return Err(invalid(
                    "coordinates must be finite and authored zoom positive",
                ));
            }
            let (x, y) = user_axes(geometry(dest.page)?, x, y);
            if !finite(x) || !finite(y) {
                return Err(invalid("coordinate transformation overflow"));
            }
            array.extend([
                Object::Name(b"XYZ".to_vec()),
                number(x),
                number(y),
                number(zoom),
            ]);
        }
        DestinationView::FitWidth { x, y } => {
            if !finite(x) || !finite(y) {
                return Err(invalid("non-finite FitH coordinate"));
            }
            let (x, y) = user_axes(geometry(dest.page)?, x, y);
            if x.is_some() || !finite(y) {
                return Err(invalid("FitH cannot represent this displayed axis"));
            }
            array.extend([Object::Name(b"FitH".to_vec()), number(y)]);
        }
    }
    Ok(Object::Array(array))
}

impl Document<'_> {
    pub(crate) fn link_target_geometry(
        &self,
        target: &LinkTarget,
    ) -> Result<Option<(usize, PageGeometry)>, EngineError> {
        let page = match target {
            LinkTarget::Page(page) => Some(*page),
            LinkTarget::Destination(destination) => Some(destination.page),
            LinkTarget::Named(_) => self
                .resolve_link_target(target)?
                .map(|destination| destination.page),
            _ => None,
        };
        page.map(|page| self.page_geometry(page).map(|geometry| (page, geometry)))
            .transpose()
    }

    pub fn named_link_destinations(&self) -> Result<Vec<String>, EngineError> {
        let pdf = crate::security::load(&self.snapshot()?, self.password())?;
        Resolver::new(&pdf).names()
    }

    pub fn resolve_link_target(
        &self,
        target: &LinkTarget,
    ) -> Result<Option<Destination>, EngineError> {
        if matches!(
            target,
            LinkTarget::Uri(_) | LinkTarget::RemotePage(_) | LinkTarget::Unsupported
        ) {
            return Ok(None);
        }
        let pdf = crate::security::load(&self.snapshot()?, self.password())?;
        let object = authored(&pdf, target, &Dictionary::new(), &|page| {
            self.page_geometry(page)
        })?;
        Resolver::new(&pdf)
            .destination(&object, &|page| self.page_geometry(page))
            .map(Some)
    }
}

#[cfg(test)]
#[path = "link_destination_tests.rs"]
mod tests;
