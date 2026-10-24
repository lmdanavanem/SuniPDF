//! Readable, selectable-text report pages; only newly created documents are rendered here.

use crate::issue_report::{
    cancelled, display, invalid, IssueReportModel, IssueReportPhase, IssueReportProgress,
    LegendStyle, MAX_REPORT_BYTES, MAX_REPORT_PAGES, MAX_TEXT_BYTES,
};
use crate::{font_embed, EngineError};
use pdfium_render::prelude::*;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::AtomicBool;

const UBUNTU: &[u8] = include_bytes!("../tests/fixtures/fonts/Ubuntu-Light.ttf");
const NOTO: &[u8] = include_bytes!("../tests/fixtures/fonts/NotoSansJP-Regular.otf");
const WIDTH: f32 = 595.0;
const HEIGHT: f32 = 842.0;
const MARGIN: f32 = 42.0;
const BODY: f32 = 9.0;

struct Block {
    text: String,
    heading: bool,
    indent: usize,
    sample: Option<(String, LegendStyle)>,
}
struct Line {
    text: String,
    heading: bool,
    indent: usize,
    sample: Option<(String, LegendStyle)>,
}

fn visible(value: &str) -> String {
    value
        .chars()
        .flat_map(|c| {
            if c.is_control() {
                c.escape_default().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}
fn push(blocks: &mut Vec<Block>, text: String, heading: bool, indent: usize) {
    blocks.push(Block {
        text,
        heading,
        indent,
        sample: None,
    });
}
fn flatten(
    blocks: &mut Vec<Block>,
    name: &str,
    value: &Value,
    depth: usize,
) -> Result<(), EngineError> {
    if depth > 16 {
        return Err(invalid("report metadata nesting exceeds 16 levels"));
    }
    match value {
        Value::Object(values) if !values.is_empty() => {
            if !name.is_empty() {
                push(blocks, visible(name), false, depth);
            }
            let scalar = values
                .iter()
                .filter(|(_, v)| !v.is_object() && !v.is_array())
                .map(|(k, v)| {
                    format!(
                        "{}: {}",
                        visible(k),
                        if v.is_null() {
                            "Not set".into()
                        } else {
                            visible(&display(v))
                        }
                    )
                })
                .collect::<Vec<_>>();
            if !scalar.is_empty() {
                push(blocks, scalar.join("; "), false, depth + 1);
            }
            for (key, value) in values.iter().filter(|(_, v)| v.is_object() || v.is_array()) {
                flatten(blocks, key, value, depth + 1)?;
            }
        }
        Value::Array(values) if name == "members" => {
            let members = values
                .iter()
                .map(|v| {
                    format!(
                        "page {}/index {}/{}",
                        v["page"],
                        v["index"],
                        visible(&display(&v["annotation_id"]))
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            push(blocks, format!("Members: {members}"), false, depth);
        }
        Value::Array(values) if values.iter().all(|v| !v.is_object() && !v.is_array()) => push(
            blocks,
            format!("{}: {}", visible(name), visible(&value.to_string())),
            false,
            depth,
        ),
        Value::Array(values) if !values.is_empty() => {
            push(
                blocks,
                format!("{} ({} entries)", visible(name), values.len()),
                false,
                depth,
            );
            for (i, value) in values.iter().enumerate() {
                flatten(blocks, &format!("{}", i + 1), value, depth + 1)?;
            }
        }
        value => push(
            blocks,
            format!(
                "{}: {}",
                visible(name),
                if value.is_null() {
                    "Not set".into()
                } else {
                    visible(&display(value))
                }
            ),
            false,
            depth,
        ),
    }
    Ok(())
}

fn blocks(model: &IssueReportModel, cancel: &AtomicBool) -> Result<Vec<Block>, EngineError> {
    let mut output = Vec::new();
    push(&mut output, model.title.clone(), true, 0);
    push(
        &mut output,
        format!("Selection source: {}", model.selection.id()),
        false,
        0,
    );
    push(
        &mut output,
        format!(
            "Coverage: {} | {} selected annotations",
            if model.complete {
                "complete"
            } else {
                "incomplete; see warnings"
            },
            model.quantities["rows"].as_array().map_or(0, Vec::len)
        ),
        false,
        0,
    );
    push(&mut output, "Report provenance".into(), true, 0);
    flatten(
        &mut output,
        "",
        &serde_json::to_value(&model.provenance).expect("provenance"),
        0,
    )?;
    for warning in &model.warnings {
        push(&mut output, format!("Warning: {warning}"), false, 0);
    }
    push(
        &mut output,
        "Selection and calculation settings".into(),
        true,
        0,
    );
    let mut options = model.quantities.clone();
    for key in ["rows", "summary", "formula_rows"] {
        options
            .as_object_mut()
            .expect("quantity export")
            .remove(key);
    }
    flatten(&mut output, "", &options, 0)?;
    push(&mut output, "Quantity and cost summary".into(), true, 0);
    flatten(&mut output, "", &model.quantities["summary"], 0)?;
    push(&mut output, "Issue groups".into(), true, 0);
    if model.issue_groups.is_empty() {
        push(
            &mut output,
            "No annotations match the selected filter.".into(),
            false,
            0,
        );
    }
    for group in &model.issue_groups {
        cancelled(cancel)?;
        flatten(
            &mut output,
            "Group",
            &serde_json::to_value(group).expect("group"),
            0,
        )?;
    }
    if model.legend_included {
        push(
            &mut output,
            "Legend of selected annotations".into(),
            true,
            0,
        );
        push(&mut output,"Styles below are the supported declarations stored in the PDF. Unknown appearances are not reconstructed.".into(),false,0);
        if model.legend.is_empty() {
            push(
                &mut output,
                "No legend entries match the selected filter.".into(),
                false,
                0,
            );
        }
        for entry in &model.legend {
            cancelled(cancel)?;
            if let Some(style) = &entry.style {
                output.push(Block {
                    text: format!(
                        "{} / {} / {} annotations / declared style sample at 1:5",
                        entry.id, entry.kind, entry.count
                    ),
                    heading: false,
                    indent: 8,
                    sample: Some((entry.kind.clone(), style.clone())),
                });
            }
            flatten(
                &mut output,
                &entry.id,
                &serde_json::to_value(entry).expect("legend"),
                0,
            )?;
        }
    }
    push(&mut output, "Annotation details".into(), true, 0);
    let formulas = model.quantities["formula_rows"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|value| {
            (
                (value["page"].as_u64(), value["index"].as_u64()),
                &value["values"],
            )
        })
        .collect::<BTreeMap<_, _>>();
    if let Some(rows) = model.quantities["rows"].as_array() {
        for row in rows {
            cancelled(cancel)?;
            push(
                &mut output,
                format!(
                    "Page {} / annotation {} / {}",
                    row["register"]["page"],
                    row["register"]["index"],
                    visible(&display(&row["register"]["kind"]))
                ),
                true,
                0,
            );
            flatten(&mut output, "", row, 0)?;
            if let Some(values) = formulas.get(&(
                row["register"]["page"].as_u64(),
                row["register"]["index"].as_u64(),
            )) {
                flatten(&mut output, "Calculated fields", values, 0)?;
            }
        }
    }
    let bytes = output.iter().try_fold(0usize, |n, b| {
        n.checked_add(b.text.len())
            .ok_or_else(|| invalid("report text overflow"))
    })?;
    if bytes > MAX_TEXT_BYTES {
        return Err(invalid("report text exceeds 4 MiB; narrow the filter"));
    }
    Ok(output)
}

fn complex(c: char) -> bool {
    matches!(c as u32,0x0300..=0x036F|0x0590..=0x109F|0x1100..=0x11FF|0x1780..=0x18AF|0x1AB0..=0x1AFF|0x1DC0..=0x1DFF|0x200C..=0x200F|0x202A..=0x202E|0x2066..=0x2069|0x20D0..=0x20FF|0x3099..=0x309A|0xA960..=0xA97F|0xD7B0..=0xD7FF|0xFE00..=0xFE0F|0xFE20..=0xFE2F|0xE0100..=0xE01EF)
}

fn wrapped(
    probe: &mut PdfPage<'_>,
    text: &str,
    width: f32,
    cancel: &AtomicBool,
) -> Result<Vec<String>, EngineError> {
    let chars = text.chars().collect::<Vec<_>>();
    let mut result = Vec::new();
    let mut offset = 0;
    while offset < chars.len() {
        cancelled(cancel)?;
        let mut low = 1usize;
        let mut high = (chars.len() - offset).min(256);
        let mut accepted = 0;
        while low <= high {
            let middle = low + (high - low) / 2;
            let candidate = chars[offset..offset + middle].iter().collect::<String>();
            let measured = if candidate.trim().is_empty() {
                0.0
            } else {
                crate::paragraph::measure(probe, &candidate)?
            };
            if measured <= width {
                accepted = middle;
                low = middle + 1;
            } else {
                high = middle - 1;
            }
        }
        if accepted == 0 {
            return Err(invalid("a report glyph does not fit the printable width"));
        }
        if offset + accepted < chars.len() {
            if let Some(space) = chars[offset..offset + accepted]
                .iter()
                .rposition(|c| *c == ' ')
            {
                if space > accepted / 2 {
                    accepted = space + 1;
                }
            }
        }
        result.push(chars[offset..offset + accepted].iter().collect());
        offset += accepted;
        if result.len() > MAX_REPORT_PAGES * 50 {
            return Err(invalid("report exceeds the page limit"));
        }
    }
    if result.is_empty() {
        result.push(String::new());
    }
    Ok(result)
}

pub(crate) fn render(
    pdfium: &Pdfium,
    model: &IssueReportModel,
    cancel: &AtomicBool,
    progress: &mut impl FnMut(IssueReportProgress),
) -> Result<(Vec<u8>, usize, String), EngineError> {
    cancelled(cancel)?;
    let blocks = blocks(model, cancel)?;
    let repertoire = blocks
        .iter()
        .flat_map(|b| b.text.chars())
        .chain("SuniPDF | Page 0123456789 /".chars())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<String>();
    if repertoire.chars().any(complex) {
        return Err(invalid("report PDF supports horizontal Latin/CJK text; combining, joining and bidirectional text requires a shaping renderer"));
    }
    let (font, font_name) = match font_embed::prepare(UBUNTU, 0, &repertoire) {
        Ok(_) => (
            font_embed::PreparedFont {
                bytes: UBUNTU.to_vec(),
                is_cff: false,
            },
            "Ubuntu Light (Ubuntu Font Licence 1.0)",
        ),
        Err(EngineError::MissingGlyphs(_)) => (
            font_embed::prepare(NOTO, 0, &repertoire)?,
            "Noto Sans JP Regular (SIL Open Font License 1.1)",
        ),
        Err(error) => return Err(error),
    };
    let mut pdf = pdfium.create_new_pdf()?;
    let token = if font.is_cff {
        pdf.fonts_mut().load_type1_from_bytes(&font.bytes, true)?
    } else {
        pdf.fonts_mut()
            .load_true_type_from_bytes(&font.bytes, true)?
    };
    {
        let mut probe = pdf
            .pages_mut()
            .create_page_at_end(PdfPagePaperSize::Custom(
                PdfPoints::new(WIDTH),
                PdfPoints::new(HEIGHT),
            ))?;
        probe.objects_mut().add_text_object(PdfPageTextObject::new(
            &pdf,
            "Report",
            token,
            PdfPoints::new(BODY),
        )?)?;
    }
    let mut pages = vec![Vec::<Line>::new()];
    let mut used = 0.0;
    {
        let mut probe = pdf.pages().get(0)?;
        for (index, block) in blocks.iter().enumerate() {
            cancelled(cancel)?;
            progress(IssueReportProgress {
                phase: IssueReportPhase::Layout,
                completed: index,
                total: blocks.len(),
            });
            // Measure at the body font size and scale the available width for headings.
            let size = if block.heading { 13.0 } else { BODY };
            let indent = block.indent.min(8);
            let width = (WIDTH - 2.0 * MARGIN - indent as f32 * 9.0) * BODY / size;
            for (part, text) in wrapped(&mut probe, &block.text, width, cancel)?
                .into_iter()
                .enumerate()
            {
                let sample = if part == 0 {
                    block.sample.clone()
                } else {
                    None
                };
                let height = if sample.is_some() {
                    38.0
                } else if block.heading {
                    22.0
                } else {
                    13.0
                };
                if used + height > HEIGHT - 2.0 * MARGIN - 28.0 {
                    pages.push(Vec::new());
                    used = 0.0;
                    if pages.len() > MAX_REPORT_PAGES {
                        return Err(invalid("PDF exceeds 250 pages; narrow the filter"));
                    }
                }
                pages.last_mut().expect("page").push(Line {
                    text,
                    heading: block.heading,
                    indent,
                    sample,
                });
                used += height;
            }
        }
    }
    pdf.pages().get(0)?.delete()?;
    let mut samples = Vec::new();
    for (index, lines) in pages.iter().enumerate() {
        cancelled(cancel)?;
        progress(IssueReportProgress {
            phase: IssueReportPhase::Rendering,
            completed: index,
            total: pages.len(),
        });
        let mut page = pdf
            .pages_mut()
            .create_page_at_end(PdfPagePaperSize::Custom(
                PdfPoints::new(WIDTH),
                PdfPoints::new(HEIGHT),
            ))?;
        let mut y = HEIGHT - MARGIN - 13.0;
        for line in lines {
            cancelled(cancel)?;
            if !line.text.trim().is_empty() {
                let size = if line.heading { 13.0 } else { BODY };
                let mut object =
                    PdfPageTextObject::new(&pdf, &line.text, token, PdfPoints::new(size))?;
                object.set_fill_color(if line.heading {
                    PdfColor::new(30, 58, 85, 255)
                } else {
                    PdfColor::new(26, 31, 38, 255)
                })?;
                object.translate(
                    PdfPoints::new(MARGIN + line.indent as f32 * 9.0),
                    PdfPoints::new(y),
                )?;
                let added = page.objects_mut().add_text_object(object)?;
                let back = added
                    .as_text_object()
                    .ok_or_else(|| invalid("report text object was lost"))?
                    .text();
                let compact =
                    |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
                if compact(&back) != compact(&line.text) {
                    return Err(invalid("report text failed native Unicode readback"));
                }
            }
            if let Some((kind, style)) = &line.sample {
                samples.push((index, y - 10.0, kind.clone(), style.clone()));
            }
            y -= if line.sample.is_some() {
                38.0
            } else if line.heading {
                22.0
            } else {
                13.0
            };
        }
        let mut footer = PdfPageTextObject::new(
            &pdf,
            format!("SuniPDF | Page {} / {}", index + 1, pages.len()),
            token,
            PdfPoints::new(8.0),
        )?;
        footer.translate(PdfPoints::new(MARGIN), PdfPoints::new(24.0))?;
        page.objects_mut().add_text_object(footer)?;
        crate::edit::regenerate_content(&mut page)?;
    }
    cancelled(cancel)?;
    let mut bytes = pdf.save_to_bytes()?;
    if !samples.is_empty() {
        bytes = draw_samples(&bytes, &samples, cancel)?;
    }
    bytes = retain_font_notice(&bytes, font_name.starts_with("Ubuntu"))?;
    if bytes.len() > MAX_REPORT_BYTES {
        return Err(invalid("PDF exceeds 32 MiB; narrow the filter"));
    }
    Ok((bytes, pages.len(), font_name.into()))
}

fn draw_samples(
    bytes: &[u8],
    samples: &[(usize, f32, String, LegendStyle)],
    cancel: &AtomicBool,
) -> Result<Vec<u8>, EngineError> {
    use lopdf::{dictionary, Object, Stream};
    let mut pdf = lopdf::Document::load_mem(bytes).map_err(crate::security::lopdf_err)?;
    let pages = pdf.get_pages();
    for (index, (page, y, kind, style)) in samples.iter().enumerate() {
        cancelled(cancel)?;
        let s = crate::annot_write::AnnotStyle {
            rgb: (
                style.stroke_rgb[0],
                style.stroke_rgb[1],
                style.stroke_rgb[2],
            ),
            fill_rgb: style.fill_rgb.map(|c| (c[0], c[1], c[2])),
            width_pt: style.width_pt,
            opacity: style.opacity,
            dash_pattern: style.dash_pattern.clone(),
        };
        let ap = match kind.as_str() {
            "Square" => crate::appearance::square_ap(270.0, 120.0, &s),
            "Circle" => crate::appearance::circle_ap(270.0, 120.0, &s),
            "Line" => crate::appearance::line_ap(
                270.0,
                120.0,
                (30.0, 60.0),
                (240.0, 60.0),
                style.arrow,
                &s,
            ),
            "Ink" => crate::appearance::ink_ap(
                270.0,
                120.0,
                &[vec![
                    (30.0, 40.0),
                    (100.0, 80.0),
                    (170.0, 40.0),
                    (240.0, 80.0),
                ]],
                &s,
            ),
            _ => return Err(invalid("unsupported legend sample")),
        };
        let page_id = *pages
            .get(&(*page as u32 + 1))
            .ok_or_else(|| invalid("legend page disappeared"))?;
        let key = format!("ReportStyle{index}");
        let state = pdf
            .add_object(dictionary! {"Type"=>"ExtGState","CA"=>style.opacity,"ca"=>style.opacity});
        let page_dict = pdf
            .get_dictionary(page_id)
            .map_err(crate::security::lopdf_err)?
            .clone();
        let mut resources = match page_dict
            .get(b"Resources")
            .map_err(crate::security::lopdf_err)?
        {
            Object::Reference(id) => pdf
                .get_dictionary(*id)
                .map_err(crate::security::lopdf_err)?
                .clone(),
            Object::Dictionary(d) => d.clone(),
            _ => return Err(invalid("invalid report resources")),
        };
        let mut states = match resources.get(b"ExtGState") {
            Ok(Object::Reference(id)) => pdf
                .get_dictionary(*id)
                .map_err(crate::security::lopdf_err)?
                .clone(),
            Ok(Object::Dictionary(d)) => d.clone(),
            Err(_) => lopdf::Dictionary::new(),
            _ => return Err(invalid("invalid report graphics state")),
        };
        if states.has(key.as_bytes()) {
            return Err(invalid("legend graphics state collision"));
        }
        states.set(key.clone(), state);
        resources.set("ExtGState", states);
        let appearance = String::from_utf8(ap.content)
            .map_err(|_| invalid("invalid legend appearance content"))?
            .replace("/GS gs", &format!("/{key} gs"));
        let mut content = format!("q 0.2 0 0 0.2 {MARGIN} {y} cm /{key} gs ").into_bytes();
        content.extend_from_slice(appearance.as_bytes());
        content.extend_from_slice(b" Q\n");
        let stream = pdf.add_object(Stream::new(dictionary! {}, content));
        // PDFium may store /Contents as an indirect array. Appending its reference
        // inside a new array would hide all existing text behind an invalid nested
        // array. Resolve the container while retaining each original stream.
        let mut contents = match page_dict.get(b"Contents") {
            Ok(Object::Array(items)) => items.clone(),
            Ok(Object::Reference(id)) => {
                match pdf.get_object(*id).map_err(crate::security::lopdf_err)? {
                    Object::Array(items) => items.clone(),
                    Object::Stream(_) => vec![Object::Reference(*id)],
                    _ => return Err(invalid("invalid report page contents")),
                }
            }
            Err(_) => Vec::new(),
            _ => return Err(invalid("invalid report page contents")),
        };
        for item in &contents {
            let id = item
                .as_reference()
                .map_err(|_| invalid("report content array must contain stream references"))?;
            pdf.get_object(id)
                .and_then(Object::as_stream)
                .map_err(|_| invalid("report content array contains a non-stream object"))?;
        }
        contents.push(stream.into());
        let page = pdf
            .get_dictionary_mut(page_id)
            .map_err(crate::security::lopdf_err)?;
        page.set("Resources", resources);
        page.set("Contents", contents);
    }
    let mut out = Vec::new();
    pdf.save_to(&mut out)?;
    Ok(out)
}

fn retain_font_notice(bytes: &[u8], ubuntu: bool) -> Result<Vec<u8>, EngineError> {
    let mut pdf = lopdf::Document::load_mem(bytes).map_err(crate::security::lopdf_err)?;
    let mut info = pdf
        .trailer
        .get(b"Info")
        .ok()
        .and_then(|o| o.as_reference().ok())
        .and_then(|id| pdf.get_dictionary(id).ok())
        .cloned()
        .unwrap_or_default();
    let (notice, licence, embedding) = if ubuntu {
        (
            "Copyright 2011 Canonical Ltd. Licensed under the Ubuntu Font Licence 1.0",
            include_str!("../tests/fixtures/fonts/UFL.txt"),
            "Complete original Ubuntu Light font program, unmodified.",
        )
    } else {
        (
            "Copyright 2018 The Noto Project Authors (github.com/googlei18n/noto-fonts)",
            include_str!("../tests/fixtures/fonts/NotoSansJP-OFL.txt"),
            "Noto Sans JP Regular subset with identity-CID mapping for selectable PDF text.",
        )
    };
    info.set("ReportFontCopyright", lopdf::text_string(notice));
    info.set("ReportFontLicence", lopdf::text_string(licence));
    info.set("ReportFontEmbedding", lopdf::text_string(embedding));
    let id = pdf.add_object(info);
    pdf.trailer.set("Info", id);
    let mut out = Vec::new();
    pdf.save_to(&mut out)?;
    Ok(out)
}
