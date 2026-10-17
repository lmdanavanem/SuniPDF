//! Presse-papiers d'objets : un clip est un PDF d'une page en mémoire qui ne contient que les
//! objets copiés (polices, images et ressources comprises). Le collage conserve les objets natifs
//! pris en charge ou importe explicitement un groupe, y compris d'un autre document. Aucun état
//! PDFium n'est gardé entre la copie et le collage.

use crate::edit::checked_indices;
use crate::{Document, EngineError, PointRect};
use lopdf::{Dictionary, Object, ObjectId};
use pdfium_render::prelude::*;
use std::collections::{BTreeSet, HashMap, HashSet};

type UsedResources = BTreeSet<(Vec<u8>, Vec<u8>)>;

fn native_unsupported() -> EngineError {
    EngineError::Other("editable paste supports native text, images and paths without forms, layers, patterns or complex blending; use Paste as group for other content".into())
}

fn native_stage(stage: &str) -> EngineError {
    EngineError::Other(format!("{stage}: {}", native_unsupported()))
}

fn effective_resources(pdf: &lopdf::Document, page: ObjectId) -> Result<Dictionary, EngineError> {
    let (direct, inherited) = pdf
        .get_page_resources(page)
        .map_err(crate::security::lopdf_err)?;
    match direct {
        Some(resources) => Ok(resources.clone()),
        None => match inherited.first() {
            Some(id) => Ok(pdf
                .get_dictionary(*id)
                .map_err(crate::security::lopdf_err)?
                .clone()),
            None => Ok(Dictionary::new()),
        },
    }
}

fn plain_page(
    pdf: &lopdf::Document,
    page: ObjectId,
    resources: &Dictionary,
) -> Result<(), EngineError> {
    let page = pdf
        .get_dictionary(page)
        .map_err(crate::security::lopdf_err)?;
    if page.has(b"Group")
        || page
            .get(b"UserUnit")
            .is_ok_and(|v| v.as_float().ok() != Some(1.0))
    {
        return Err(native_unsupported());
    }
    if let Ok(spaces) = resources.get(b"ColorSpace") {
        let (_, spaces) = pdf
            .dereference(spaces)
            .map_err(crate::security::lopdf_err)?;
        let spaces = spaces.as_dict().map_err(crate::security::lopdf_err)?;
        if [b"DefaultRGB".as_slice(), b"DefaultCMYK", b"DefaultGray"]
            .iter()
            .any(|key| spaces.has(key))
        {
            return Err(native_unsupported());
        }
    }
    Ok(())
}

fn image_space(pdf: &lopdf::Document, value: &Object, depth: usize) -> Result<(), EngineError> {
    if depth >= 8 {
        return Err(native_unsupported());
    }
    let (_, value) = pdf.dereference(value).map_err(crate::security::lopdf_err)?;
    match value {
        Object::Name(name)
            if matches!(
                name.as_slice(),
                b"DeviceRGB" | b"DeviceGray" | b"DeviceCMYK"
            ) =>
        {
            Ok(())
        }
        Object::Array(values) => match values.first().and_then(|value| value.as_name().ok()) {
            Some(b"CalGray" | b"CalRGB" | b"Lab") if values.len() == 2 => Ok(()),
            Some(b"ICCBased") if values.len() == 2 => {
                let (_, profile) = pdf
                    .dereference(&values[1])
                    .map_err(crate::security::lopdf_err)?;
                let profile = profile.as_stream().map_err(crate::security::lopdf_err)?;
                if let Ok(alternate) = profile.dict.get(b"Alternate") {
                    image_space(pdf, alternate, depth + 1)?;
                }
                Ok(())
            }
            Some(b"Indexed") if values.len() == 4 => image_space(pdf, &values[1], depth + 1),
            _ => Err(native_unsupported()),
        },
        _ => Err(native_unsupported()),
    }
}

// shortcut: bound imported graphs before the existing cycle-aware copier; native paste never falls back silently.
fn bounded_graph(pdf: &lopdf::Document, values: &[Object]) -> Result<(), EngineError> {
    let mut stack: Vec<_> = values.iter().map(|value| (value, 0usize)).collect();
    let mut seen = HashSet::new();
    let mut nodes = 0usize;
    let mut bytes = 0usize;
    while let Some((value, depth)) = stack.pop() {
        nodes += 1;
        if nodes > 100_000 || depth >= 32 {
            return Err(native_unsupported());
        }
        let dictionary = match value {
            Object::Reference(id) if seen.insert(*id) => {
                stack.push((
                    pdf.objects.get(id).ok_or_else(native_unsupported)?,
                    depth + 1,
                ));
                None
            }
            Object::Array(values) => {
                if stack.len().saturating_add(values.len()) > 100_000 {
                    return Err(native_unsupported());
                }
                stack.extend(values.iter().map(|value| (value, depth + 1)));
                None
            }
            Object::Dictionary(dictionary) => Some(dictionary),
            Object::Stream(stream) => {
                bytes = bytes.saturating_add(stream.content.len());
                Some(&stream.dict)
            }
            Object::String(value, _) | Object::Name(value) => {
                bytes = bytes.saturating_add(value.len());
                None
            }
            Object::Real(value) if !value.is_finite() => return Err(native_unsupported()),
            _ => None,
        };
        if bytes > 64 * 1024 * 1024 {
            return Err(native_unsupported());
        }
        if let Some(dictionary) = dictionary {
            if dictionary
                .get(b"Type")
                .and_then(Object::as_name)
                .is_ok_and(|name| matches!(name, b"Page" | b"Pages" | b"Catalog"))
                || dictionary
                    .get(b"Subtype")
                    .and_then(Object::as_name)
                    .is_ok_and(|name| matches!(name, b"Type3" | b"Form" | b"PS"))
                || dictionary.has(b"OC")
            {
                return Err(native_unsupported());
            }
            if dictionary.get(b"Subtype").and_then(Object::as_name).ok()
                == Some(b"Image".as_slice())
            {
                if let Ok(space) = dictionary.get(b"ColorSpace") {
                    image_space(pdf, space, 0)?;
                }
            }
            if stack.len().saturating_add(dictionary.len()) > 100_000 {
                return Err(native_unsupported());
            }
            stack.extend(dictionary.iter().map(|(_, value)| (value, depth + 1)));
        }
    }
    Ok(())
}

fn validate_resource(
    pdf: &lopdf::Document,
    category: &[u8],
    value: &Object,
) -> Result<(), EngineError> {
    let (_, value) = pdf.dereference(value).map_err(crate::security::lopdf_err)?;
    match category {
        b"Font" => {
            let font = value.as_dict().map_err(crate::security::lopdf_err)?;
            if !font
                .get(b"Subtype")
                .and_then(Object::as_name)
                .is_ok_and(|kind| matches!(kind, b"Type1" | b"TrueType" | b"Type0"))
            {
                return Err(native_unsupported());
            }
        }
        b"XObject" => {
            let image = value.as_stream().map_err(crate::security::lopdf_err)?;
            if image.dict.get(b"Subtype").and_then(Object::as_name).ok()
                != Some(b"Image".as_slice())
            {
                return Err(native_unsupported());
            }
            if let Ok(space) = image.dict.get(b"ColorSpace") {
                image_space(pdf, space, 0)?;
            }
        }
        b"ExtGState" => {
            for (key, value) in value.as_dict().map_err(crate::security::lopdf_err)?.iter() {
                if key == b"BM" {
                    if value.as_name().ok() != Some(b"Normal".as_slice()) {
                        return Err(native_unsupported());
                    }
                } else if !matches!(
                    key.as_slice(),
                    b"Type"
                        | b"ca"
                        | b"CA"
                        | b"LW"
                        | b"LC"
                        | b"LJ"
                        | b"ML"
                        | b"D"
                        | b"RI"
                        | b"SA"
                        | b"SM"
                        | b"TK"
                ) {
                    return Err(native_unsupported());
                }
            }
        }
        _ => return Err(native_unsupported()),
    }
    Ok(())
}

fn balanced(
    content: &lopdf::content::Content<Vec<lopdf::content::Operation>>,
    native: bool,
) -> Result<UsedResources, EngineError> {
    let mut used = BTreeSet::new();
    let mut depth = 0usize;
    let mut operands = 0usize;
    let mut in_text = false;
    if content.operations.len() > 100_000 {
        return Err(native_unsupported());
    }
    for op in &content.operations {
        operands = operands.saturating_add(op.operands.len());
        if operands > 100_000 {
            return Err(native_unsupported());
        }
        match op.operator.as_str() {
            "q" => {
                depth += 1;
                if depth > 64 {
                    return Err(native_unsupported());
                }
            }
            "Q" => {
                depth = depth.checked_sub(1).ok_or_else(native_unsupported)?;
            }
            "BT" if !in_text => in_text = true,
            "ET" if in_text => in_text = false,
            "BT" | "ET" => return Err(native_unsupported()),
            _ => {}
        }
        if !native {
            continue;
        }
        let category: Option<&[u8]> = match op.operator.as_str() {
            "Tf" => Some(b"Font"),
            "Do" => Some(b"XObject"),
            "gs" => Some(b"ExtGState"),
            "CS" | "cs" => {
                if op.operands.len() != 1
                    || !op.operands[0].as_name().is_ok_and(|name| {
                        matches!(name, b"DeviceRGB" | b"DeviceGray" | b"DeviceCMYK")
                    })
                {
                    return Err(native_unsupported());
                }
                None
            }
            "q" | "Q" | "cm" | "w" | "J" | "j" | "M" | "d" | "ri" | "m" | "l" | "c" | "v" | "y"
            | "h" | "re" | "S" | "s" | "f" | "F" | "f*" | "B" | "B*" | "b" | "b*" | "n" | "W"
            | "W*" | "BT" | "ET" | "Tc" | "Tw" | "Tz" | "TL" | "Tr" | "Ts" | "Td" | "TD" | "Tm"
            | "T*" | "Tj" | "TJ" | "'" | "\"" | "G" | "g" | "RG" | "rg" | "K" | "k" => None,
            _ => return Err(native_unsupported()),
        };
        if let Some(category) = category {
            if op.operands.len() != if category == b"Font" { 2 } else { 1 } {
                return Err(native_unsupported());
            }
            let name = op.operands[0]
                .as_name()
                .map_err(crate::security::lopdf_err)?;
            used.insert((category.to_vec(), name.to_vec()));
            if used.len() > 4096 {
                return Err(native_unsupported());
            }
        }
    }
    if depth != 0 || in_text {
        return Err(native_unsupported());
    }
    Ok(used)
}

fn same_native_object(new: &crate::ObjectInfo, old: &crate::ObjectInfo, dx: f32, dy: f32) -> bool {
    new.kind == old.kind
        && new.text == old.text
        && new.font_name == old.font_name
        && new.fill_rgb == old.fill_rgb
        && new.stroke_rgb == old.stroke_rgb
        && new.opacity == old.opacity
        && [
            new.font_size - old.font_size,
            new.stroke_width - old.stroke_width,
            (new.angle_deg - old.angle_deg + 180.0).rem_euclid(360.0) - 180.0,
            new.bounds.left - old.bounds.left - dx,
            new.bounds.right - old.bounds.right - dx,
            new.bounds.bottom - old.bounds.bottom - dy,
            new.bounds.top - old.bounds.top - dy,
        ]
        .iter()
        .all(|delta| delta.is_finite() && delta.abs() <= 0.05)
        && new.quad.iter().all(|new| {
            old.quad.iter().any(|old| {
                [new.0 - old.0 - dx, new.1 - old.1 - dy]
                    .iter()
                    .all(|delta| delta.is_finite() && delta.abs() <= 0.05)
            })
        })
}

fn native_source_supported(
    bytes: &[u8],
    password: Option<&str>,
    page_index: usize,
) -> Result<(), EngineError> {
    if bytes.len() > 64 * 1024 * 1024 {
        return Err(native_unsupported());
    }
    let pdf = crate::security::load(bytes, password)?;
    let catalog = pdf.catalog().map_err(crate::security::lopdf_err)?;
    if catalog.has(b"OCProperties") || catalog.has(b"StructTreeRoot") {
        return Err(native_unsupported());
    }
    let page_number = u32::try_from(page_index)
        .ok()
        .and_then(|index| index.checked_add(1))
        .ok_or_else(native_unsupported)?;
    let page = *pdf
        .get_pages()
        .get(&page_number)
        .ok_or_else(native_unsupported)?;
    let resources = effective_resources(&pdf, page)?;
    plain_page(&pdf, page, &resources).map_err(|_| native_stage("source page resources"))?;
    let content = lopdf::content::Content::decode_strict(
        &pdf.get_page_content_with_limit(page, 32 * 1024 * 1024)
            .map_err(crate::security::lopdf_err)?,
    )
    .map_err(crate::security::lopdf_err)?;
    let mut values = Vec::new();
    for (category, name) in
        balanced(&content, true).map_err(|_| native_stage("source content operators"))?
    {
        let (_, entries) = pdf
            .dereference(
                resources
                    .get(&category)
                    .map_err(crate::security::lopdf_err)?,
            )
            .map_err(crate::security::lopdf_err)?;
        let value = entries
            .as_dict()
            .map_err(crate::security::lopdf_err)?
            .get(&name)
            .map_err(crate::security::lopdf_err)?;
        validate_resource(&pdf, &category, value)
            .map_err(|_| native_stage("source font, image or graphics-state resource"))?;
        values.push(value.clone());
    }
    values.extend(
        content
            .operations
            .iter()
            .flat_map(|op| op.operands.iter().cloned()),
    );
    bounded_graph(&pdf, &values).map_err(|_| native_stage("source object graph"))
}

#[derive(Debug, Clone)]
pub struct ObjectClip {
    pdf: Vec<u8>,
    native_source_supported: bool,
    native_source_refusal: Option<String>,
    /// Nombre d'objets copiés.
    pub count: usize,
    /// Texte des objets texte copiés, une ligne par objet.
    pub text: String,
    /// Union des limites d'affichage dans la page source.
    pub bounds: PointRect,
}

impl Document<'_> {
    /// Copie des objets d'une page. Le document n'est pas modifié (ordre de dessin compris).
    pub fn copy_objects(
        &self,
        page_index: usize,
        indices: &[usize],
    ) -> Result<ObjectClip, EngineError> {
        let page = self.page_handle(page_index)?;
        let count = page.objects().len();
        drop(page);
        let keep = checked_indices(indices, count)?;
        let infos: Vec<_> = self
            .page_objects(page_index)?
            .into_iter()
            .filter(|o| keep.binary_search(&o.index).is_ok())
            .collect();
        let bounds = infos
            .iter()
            .skip(1)
            .fold(infos[0].bounds, |u, o| PointRect {
                left: u.left.min(o.bounds.left),
                bottom: u.bottom.min(o.bounds.bottom),
                right: u.right.max(o.bounds.right),
                top: u.top.max(o.bounds.top),
            });
        let text = infos
            .iter()
            .filter_map(|o| o.text.as_deref())
            .filter(|t| !t.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        // Inspect the actual source before importing or regenerating it. PDFium may rename the
        // imported page's resources before that page has been parsed and regenerated.
        let native_preflight = self
            .snapshot()
            .and_then(|bytes| native_source_supported(&bytes, self.password(), page_index));
        let mut tmp = self.pdfium().create_new_pdf()?;
        tmp.pages_mut()
            .copy_page_from_document(&self.inner, page_index as PdfPageIndex, 0)?;
        {
            let mut p = tmp.pages().get(0)?;
            // La copie relit le flux de contenu : mêmes objets, même ordre, sinon on refuse.
            if p.objects().len() != count {
                return Err(EngineError::Other(
                    "page content could not be copied".into(),
                ));
            }
            for i in (0..count).rev() {
                if keep.binary_search(&i).is_err() {
                    p.objects_mut()
                        .remove_object_at_index(i as PdfPageObjectIndex)?;
                }
            }
            crate::edit::regenerate_content(&mut p)?;
        }
        let pdf = tmp.save_to_bytes()?;
        let native_preflight =
            native_preflight.and_then(|()| native_source_supported(&pdf, None, 0));
        let native_source_supported = native_preflight.is_ok();
        let native_source_refusal = native_preflight.err().map(|error| error.to_string());
        Ok(ObjectClip {
            pdf,
            native_source_supported,
            native_source_refusal,
            count: keep.len(),
            text,
            bounds,
        })
    }
}

impl<'a> Document<'a> {
    /// Pastes supported native objects individually. Unsupported content leaves the document unchanged.
    pub fn paste_editable_objects(
        &mut self,
        page_index: usize,
        clip: &ObjectClip,
        dx: f32,
        dy: f32,
    ) -> Result<Vec<usize>, EngineError> {
        if !dx.is_finite()
            || !dy.is_finite()
            || clip.count == 0
            || clip.count > 10_000
            || clip.pdf.len() > 64 * 1024 * 1024
        {
            return Err(native_unsupported());
        }
        if !clip.native_source_supported {
            return Err(EngineError::Other(
                clip.native_source_refusal
                    .clone()
                    .unwrap_or_else(|| native_unsupported().to_string()),
            ));
        }
        let target_geo = self.page_geometry(page_index)?;
        let source = Document {
            inner: self.pdfium.load_pdf_from_byte_vec(clip.pdf.clone(), None)?,
            pdfium: self.pdfium,
            info: self.info.clone(),
            page_rotations: Vec::new(),
            password: None,
            font_cache: Default::default(),
        };
        let source_geo = source.page_geometry(0)?;
        let source_infos = source.page_objects(0)?;
        if source_infos.len() != clip.count
            || source_infos.iter().any(|o| {
                !matches!(
                    o.kind,
                    crate::ObjectKind::Text | crate::ObjectKind::Image | crate::ObjectKind::Path
                )
            })
        {
            return Err(native_unsupported());
        }
        let source_bounds =
            source_infos
                .iter()
                .skip(1)
                .fold(source_infos[0].bounds, |bounds, object| PointRect {
                    left: bounds.left.min(object.bounds.left),
                    bottom: bounds.bottom.min(object.bounds.bottom),
                    right: bounds.right.max(object.bounds.right),
                    top: bounds.top.max(object.bounds.top),
                });
        if ![
            source_bounds.left,
            source_bounds.bottom,
            source_bounds.right,
            source_bounds.top,
        ]
        .iter()
        .all(|value| value.is_finite())
        {
            return Err(native_unsupported());
        }
        let src = crate::security::load(&clip.pdf, None)?;
        let mut dst = crate::security::load(&self.snapshot()?, self.password())?;
        let source_page = *src.get_pages().get(&1).ok_or_else(native_unsupported)?;
        let target_page = *dst
            .get_pages()
            .get(&(page_index as u32 + 1))
            .ok_or(EngineError::PageOutOfRange(page_index))?;
        if src
            .catalog()
            .map_err(crate::security::lopdf_err)?
            .has(b"OCProperties")
            || src
                .catalog()
                .map_err(crate::security::lopdf_err)?
                .has(b"StructTreeRoot")
        {
            return Err(native_unsupported());
        }
        let source_resources = effective_resources(&src, source_page)?;
        let mut target_resources = effective_resources(&dst, target_page)?;
        plain_page(&src, source_page, &source_resources)?;
        plain_page(&dst, target_page, &target_resources)?;
        let mut content = lopdf::content::Content::decode_strict(
            &src.get_page_content_with_limit(source_page, 32 * 1024 * 1024)
                .map_err(crate::security::lopdf_err)?,
        )
        .map_err(crate::security::lopdf_err)?;
        let used = balanced(&content, true)?;
        let old_content = dst
            .get_page_content_with_limit(target_page, 32 * 1024 * 1024)
            .map_err(crate::security::lopdf_err)?;
        balanced(
            &lopdf::content::Content::decode_strict(&old_content)
                .map_err(crate::security::lopdf_err)?,
            false,
        )?;
        let mut values = Vec::new();
        for (category, name) in &used {
            let (_, entries) = src
                .dereference(
                    source_resources
                        .get(category)
                        .map_err(crate::security::lopdf_err)?,
                )
                .map_err(crate::security::lopdf_err)?;
            let value = entries
                .as_dict()
                .map_err(crate::security::lopdf_err)?
                .get(name)
                .map_err(crate::security::lopdf_err)?;
            validate_resource(&src, category, value)?;
            values.push(value.clone());
        }
        values.extend(
            content
                .operations
                .iter()
                .flat_map(|op| op.operands.iter().cloned()),
        );
        bounded_graph(&src, &values)?;
        let mut copied = HashMap::new();
        let mut renamed = HashMap::new();
        for ((category, name), value) in used.iter().zip(values.iter()) {
            let mut entries = match target_resources.get(category) {
                Ok(value) => {
                    let (_, value) = dst.dereference(value).map_err(crate::security::lopdf_err)?;
                    value.as_dict().map_err(crate::security::lopdf_err)?.clone()
                }
                Err(_) => Dictionary::new(),
            };
            let mut suffix = 0usize;
            let new_name = loop {
                let name = format!("SuniPaste{}_{}", dst.max_id, suffix).into_bytes();
                if !entries.has(&name) {
                    break name;
                }
                suffix += 1;
                if suffix > 4096 {
                    return Err(native_unsupported());
                }
            };
            let imported = crate::forms::deep_copy(
                &src,
                &mut dst,
                &HashMap::new(),
                &mut copied,
                0,
                value,
                32,
            )?;
            entries.set(new_name.clone(), imported);
            target_resources.set(category.clone(), entries);
            renamed.insert((category.clone(), name.clone()), new_name);
        }
        for op in &mut content.operations {
            let category: &[u8] = match op.operator.as_str() {
                "Tf" => b"Font",
                "Do" => b"XObject",
                "gs" => b"ExtGState",
                _ => continue,
            };
            let old = op.operands[0]
                .as_name()
                .map_err(crate::security::lopdf_err)?;
            op.operands[0] = Object::Name(
                renamed
                    .get(&(category.to_vec(), old.to_vec()))
                    .ok_or_else(native_unsupported)?
                    .clone(),
            );
        }
        let (w, h) = target_geo.display_size();
        let (bw, bh) = (
            source_bounds.right - source_bounds.left,
            source_bounds.top - source_bounds.bottom,
        );
        let (mut tx, mut ty) = (dx, dy);
        if source_bounds.left + dx > w
            || source_bounds.bottom + dy > h
            || source_bounds.right + dx < 0.0
            || source_bounds.top + dy < 0.0
        {
            tx = (w - bw) / 2.0 - source_bounds.left;
            ty = (h - bh) / 2.0 - source_bounds.bottom;
        }
        let (a, b) = source_geo.vec_to_display(1.0, 0.0);
        let (c, d) = source_geo.vec_to_display(0.0, 1.0);
        let (e, f) = source_geo.to_display(0.0, 0.0);
        let t = target_geo.upright_matrix(tx, ty);
        let m = PdfMatrix::new(a, b, c, d, e, f)
            .multiply(PdfMatrix::new(t[0], t[1], t[2], t[3], t[4], t[5]));
        let bounds = source_geo.bbox;
        if ![
            m.a(),
            m.b(),
            m.c(),
            m.d(),
            m.e(),
            m.f(),
            bounds.left,
            bounds.bottom,
            bounds.right,
            bounds.top,
        ]
        .iter()
        .all(|value| value.is_finite())
            || bounds.right <= bounds.left
            || bounds.top <= bounds.bottom
        {
            return Err(native_unsupported());
        }
        let prefix = format!(
            "\nQ\nq\n{} {} {} {} {} {} cm\n{} {} {} {} re W n\n",
            m.a(),
            m.b(),
            m.c(),
            m.d(),
            m.e(),
            m.f(),
            bounds.left,
            bounds.bottom,
            bounds.right - bounds.left,
            bounds.top - bounds.bottom
        );
        let mut combined = b"q\n".to_vec();
        combined.extend(old_content);
        combined.extend(prefix.as_bytes());
        combined.extend(content.encode().map_err(crate::security::lopdf_err)?);
        combined.extend(b"\nQ\n");
        let stream = dst.add_object(lopdf::Stream::new(Dictionary::new(), combined));
        let page = dst
            .get_dictionary_mut(target_page)
            .map_err(crate::security::lopdf_err)?;
        page.set("Contents", stream);
        page.set("Resources", target_resources);
        if src.version > dst.version {
            dst.version = src.version.clone();
        }
        let bytes = crate::security::save(dst)?;
        let scratch = Document {
            inner: self.pdfium.load_pdf_from_byte_vec(bytes, self.password())?,
            pdfium: self.pdfium,
            info: self.info.clone(),
            page_rotations: self.page_rotations.clone(),
            password: self.password.clone(),
            font_cache: Default::default(),
        };
        let before = self.page_objects(page_index)?;
        let after = scratch.page_objects(page_index)?;
        if after.len() != before.len() + clip.count
            || !after[..before.len()]
                .iter()
                .zip(&before)
                .all(|(new, old)| same_native_object(new, old, 0.0, 0.0))
        {
            return Err(native_stage(
                "existing destination objects changed during import",
            ));
        }
        for (new, old) in after[before.len()..].iter().zip(&source_infos) {
            if !same_native_object(new, old, tx, ty) {
                return Err(EngineError::Other(format!(
                    "native paste changed object geometry or style: source {:?}, size {}, opacity {:?}, angle {}, bounds {:?}; pasted {:?}, size {}, opacity {:?}, angle {}, bounds {:?}",
                    old.kind, old.font_size, old.opacity, old.angle_deg, old.bounds,
                    new.kind, new.font_size, new.opacity, new.angle_deg, new.bounds,
                )));
            }
        }
        let indices = (before.len()..after.len()).collect();
        self.inner = scratch.inner;
        self.font_cache.clear();
        Ok(indices)
    }

    /// Colle un clip en un seul objet groupe sur `page_index`, à la position d'origine décalée de
    /// (dx, dy) points d'affichage (recentré sur la page s'il en sortirait). Renvoie son index.
    pub fn paste_objects(
        &mut self,
        page_index: usize,
        clip: &ObjectClip,
        dx: f32,
        dy: f32,
    ) -> Result<usize, EngineError> {
        if !(dx.is_finite() && dy.is_finite()) {
            return Err(EngineError::Other("invalid offset".into()));
        }
        let geo = self.page_geometry(page_index)?;
        let src = self
            .pdfium()
            .load_pdf_from_byte_vec(clip.pdf.clone(), None)?;
        let mut source_page = src.pages().get(0)?;
        // `objects_mut()` porte la durée de vie du document (exigée par l'import inter-documents).
        let form = source_page
            .objects_mut()
            .copy_into_x_object_form_object(&mut self.inner)?;
        let mut page = self.page_handle(page_index)?;
        let mut object = page.objects_mut().add_object(form)?;
        // Le formulaire reprend la page source telle qu'affichée : le poser « droit ».
        let m = geo.upright_matrix(0.0, 0.0);
        object.transform(m[0], m[1], m[2], m[3], m[4], m[5])?;
        let index = page.objects().len() - 1;
        drop(object);
        crate::edit::regenerate_content(&mut page)?;
        drop(page);
        drop(source_page);
        drop(src);
        let now = self.object_info(page_index, index)?.bounds;
        let (w, h) = geo.display_size();
        let mut target = (clip.bounds.left + dx, clip.bounds.bottom + dy);
        let (bw, bh) = (now.right - now.left, now.top - now.bottom);
        if target.0 > w || target.1 > h || target.0 + bw < 0.0 || target.1 + bh < 0.0 {
            target = ((w - bw) / 2.0, (h - bh) / 2.0);
        }
        self.move_object(
            page_index,
            index,
            target.0 - now.left,
            target.1 - now.bottom,
        )?;
        Ok(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_resource_preflight_refuses_aliases_complex_blends_and_unbounded_graphs() {
        let mut pdf = lopdf::Document::with_version("1.7");
        assert!(image_space(&pdf, &Object::Name(b"PageLocalColorSpace".to_vec()), 0).is_err());
        let image = Object::Stream(lopdf::Stream::new(
            lopdf::dictionary! {"Subtype"=>"Image", "ColorSpace"=>"DeviceRGB"},
            vec![0, 0, 0],
        ));
        assert!(validate_resource(&pdf, b"XObject", &image).is_ok());
        let blend = Object::Dictionary(lopdf::dictionary! {"Type"=>"ExtGState", "BM"=>"Multiply"});
        assert!(validate_resource(&pdf, b"ExtGState", &blend).is_err());
        let form = Object::Stream(lopdf::Stream::new(
            lopdf::dictionary! {"Subtype"=>"Form"},
            Vec::new(),
        ));
        assert!(bounded_graph(&pdf, &[form]).is_err());
        let mut nested = Object::Integer(1);
        for _ in 0..33 {
            nested = Object::Array(vec![nested]);
        }
        assert!(bounded_graph(&pdf, &[nested]).is_err());
        assert!(bounded_graph(&pdf, &[Object::Reference((999, 0))]).is_err());
        let id = pdf.new_object_id();
        pdf.objects.insert(
            id,
            Object::Dictionary(lopdf::dictionary! {"Shared"=>Object::Reference(id)}),
        );
        assert!(bounded_graph(&pdf, &[Object::Reference(id), Object::Reference(id)]).is_ok());
        let id = pdf.add_object(lopdf::dictionary! {"Subtype"=>"Type3"});
        assert!(bounded_graph(&pdf, &[Object::Reference(id)]).is_err());
        let content = |bytes| lopdf::content::Content::decode_strict(bytes).unwrap();
        assert!(balanced(&content(b"q Q Q"), true).is_err());
        assert!(balanced(&content(b"BT /F1 12 Tf (Text) Tj ET"), true).is_ok());
        assert!(balanced(&content(b"/Spot cs 1 scn"), true).is_err());
        assert!(balanced(&content(b"/Sh1 sh"), true).is_err());
    }

    #[test]
    fn native_preflight_refuses_dangling_graphics_state_even_for_default_reset() {
        let mut pdf =
            lopdf::Document::load_mem(include_bytes!("../tests/fixtures/two-pages.pdf")).unwrap();
        let page = pdf.get_pages()[&1];
        let stream = pdf.add_object(lopdf::Stream::new(
            Dictionary::new(),
            b"q 0 0 0 RG 0 0 0 rg 1 w 0 J 0 j /MissingState gs Q".to_vec(),
        ));
        pdf.get_dictionary_mut(page)
            .unwrap()
            .set("Contents", stream);
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        assert!(native_source_supported(&bytes, None, 0).is_err());
    }
}
