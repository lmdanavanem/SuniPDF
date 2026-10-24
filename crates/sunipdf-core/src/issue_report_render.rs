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
const BODY: f32 = 10.0;

#[derive(Clone, Copy)]
enum TextStyle {
    Title,
    Section,
    Item,
    Body,
    Muted,
    Warning,
}
impl TextStyle {
    fn size(self) -> f32 {
        match self {
            Self::Title => 21.0,
            Self::Section => 13.0,
            Self::Item => 11.0,
            Self::Muted => 9.0,
            Self::Body | Self::Warning => BODY,
        }
    }
    fn height(self) -> f32 {
        self.size() + 4.5
    }
    fn gap(self) -> f32 {
        match self {
            Self::Title => 0.0,
            Self::Section => 13.0,
            Self::Item => 7.0,
            _ => 2.0,
        }
    }
    fn keep_with_next(self) -> bool {
        matches!(self, Self::Title | Self::Section | Self::Item)
    }
    fn color(self) -> PdfColor {
        match self {
            Self::Title | Self::Section | Self::Item => PdfColor::new(30, 58, 85, 255),
            Self::Muted => PdfColor::new(82, 94, 108, 255),
            Self::Warning => PdfColor::new(139, 60, 18, 255),
            Self::Body => PdfColor::new(26, 31, 38, 255),
        }
    }
}

struct Block {
    text: String,
    style: TextStyle,
    indent: usize,
    sample: Option<(String, LegendStyle)>,
    break_before: bool,
}
struct Line {
    text: String,
    style: TextStyle,
    indent: usize,
    sample: Option<(String, LegendStyle)>,
    space_before: f32,
}
impl Line {
    fn height(&self) -> f32 {
        if self.sample.is_some() {
            38.0
        } else {
            self.style.height()
        }
    }
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
    styled(
        blocks,
        text,
        if heading {
            TextStyle::Section
        } else {
            TextStyle::Muted
        },
        indent,
    );
}
fn styled(blocks: &mut Vec<Block>, text: String, style: TextStyle, indent: usize) {
    blocks.push(Block {
        text: visible(&text),
        style,
        indent,
        sample: None,
        break_before: false,
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

fn value_text(value: &Value) -> String {
    if value.is_null() {
        "Not available".into()
    } else {
        visible(&display(value))
    }
}

fn label(value: &Value) -> String {
    match value.as_str() {
        Some("geometry_unknown") => "Unknown geometry".into(),
        Some("legacy_unverified") => "Legacy measurement, unverified".into(),
        Some("missing_field") => "Missing field".into(),
        Some("non_numeric_field") => "Field is not numeric".into(),
        Some("missing_quantity") => "Missing quantity".into(),
        Some("unverified_quantity") => "Unverified quantity".into(),
        Some("unit_mismatch") => "Incompatible units".into(),
        Some("division_by_zero") => "Division by zero".into(),
        Some(text) => {
            let mut chars = text.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().collect::<String>() + &chars.as_str().replace('_', " ")
            })
        }
        None => "Not set".into(),
    }
}

fn unit(value: &Value) -> String {
    match value.as_str() {
        Some("mm2") => "mm²",
        Some("cm2") => "cm²",
        Some("m2") => "m²",
        Some("in2") => "in²",
        Some("ft2") => "ft²",
        Some("mm3") => "mm³",
        Some("cm3") => "cm³",
        Some("m3") => "m³",
        Some("in3") => "in³",
        Some("ft3") => "ft³",
        Some("deg") => "degrees",
        Some(text) => text,
        None => "dimensionless",
    }
    .into()
}

fn body(output: &mut Vec<Block>, text: impl Into<String>) {
    styled(output, text.into(), TextStyle::Body, 0);
}
fn section(output: &mut Vec<Block>, text: &str) {
    styled(output, text.into(), TextStyle::Section, 0);
}
fn item(output: &mut Vec<Block>, text: impl Into<String>) {
    styled(output, text.into(), TextStyle::Item, 0);
}
fn note(output: &mut Vec<Block>, text: impl Into<String>) {
    styled(output, text.into(), TextStyle::Muted, 0);
}
fn entries(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}
fn named(value: &Value, fallback: &str) -> String {
    value
        .as_str()
        .filter(|v| !v.is_empty())
        .map_or_else(|| fallback.into(), visible)
}
fn quantity(value: &Value) -> String {
    if value.is_null() {
        "Not available; excluded from quantity totals".into()
    } else {
        format!("{} {}", value_text(&value["value"]), unit(&value["unit"]))
    }
}
fn money(value: &Value) -> String {
    format!(
        "{} {} ({} decimal places)",
        value_text(&value["amount"]),
        value_text(&value["currency"]),
        value_text(&value["rounding_decimals"])
    )
}
fn identities(members: &[crate::issue_report::ReportRowIdentity]) -> String {
    members
        .iter()
        .map(|member| {
            format!(
                "page {}, index {}{}",
                member.page,
                member.index,
                member
                    .annotation_id
                    .as_ref()
                    .map_or(String::new(), |id| format!(" / {}", visible(id)))
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn summary(output: &mut Vec<Block>, model: &IssueReportModel, rows: &[Value]) {
    let summary = &model.quantities["summary"];
    section(output, "Report summary");
    body(
        output,
        format!(
            "{} selected annotations | {} with quantities | {} without quantities",
            rows.len(),
            value_text(&summary["quantity_rows"]),
            value_text(&summary["unquantified_rows"])
        ),
    );
    let priced = rows
        .iter()
        .filter(|row| !row["line_total"].is_null())
        .count();
    let eligible = rows
        .iter()
        .filter(|row| !row["quantity"].is_null() && row["quantity"]["unit"].as_str() != Some("deg"))
        .count();
    body(output, format!("Cost coverage: {priced} of {eligible} additive quantity annotations priced. Missing prices are not zero."));
    let mut locations = BTreeMap::<String, usize>::new();
    for row in rows {
        *locations
            .entry(label(&row["register"]["zone"]["status"]))
            .or_default() += 1;
    }
    body(
        output,
        if rows.is_empty() {
            "Selected annotation locations: no annotations selected.".into()
        } else {
            format!(
                "Selected annotation locations: {}.",
                locations
                    .iter()
                    .map(|(name, count)| format!("{name}: {count}"))
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        },
    );
    body(
        output,
        format!(
            "Measurement and calculation checks: {}.",
            if model.complete {
                "no unresolved checks in the report scope"
            } else {
                "incomplete; review the warnings below"
            }
        ),
    );
    note(output, format!("Additive quantities: {} | Angles (not summed): {} | Unverified measurements: {} (legacy: {}; stale: {})",
        value_text(&summary["additive_quantity_rows"]), value_text(&summary["angular_rows"]),
        value_text(&summary["unverified_measurement_rows"]), value_text(&summary["legacy_measurement_rows"]),
        value_text(&summary["stale_measurement_rows"])));
    for warning in &model.warnings {
        styled(
            output,
            format!("Attention: {warning}"),
            TextStyle::Warning,
            0,
        );
    }
    section(output, "Quantity and cost totals");
    note(output, "Subtotals retain their own group, unit, currency and rounding. Partial costs include priced annotations only.");
    if entries(&summary["groups"]).is_empty() {
        body(
            output,
            "No additive quantities are available for this selection.",
        );
    }
    for group in entries(&summary["groups"]) {
        item(
            output,
            format!(
                "{} | {} {}",
                named(&group["group"], "All selected annotations"),
                value_text(&group["quantity"]),
                unit(&group["unit"])
            ),
        );
        body(
            output,
            format!(
                "{} contributing annotations | {} priced | {} missing prices",
                value_text(&group["rows"]),
                value_text(&group["priced_rows"]),
                value_text(&group["missing_prices"])
            ),
        );
        for cost in entries(&group["costs"]) {
            body(
                output,
                format!(
                    "Cost: {} | {} priced annotations | {}",
                    money(&cost["total"]),
                    value_text(&cost["priced_rows"]),
                    if cost["complete"] == true {
                        "complete within this subtotal"
                    } else {
                        "partial subtotal"
                    }
                ),
            );
        }
        if entries(&group["costs"]).is_empty() {
            body(output, "Cost: not available; no priced subtotal.");
        }
    }
    if !entries(&summary["formula_groups"]).is_empty() {
        section(output, "Calculated totals");
        for group in entries(&summary["formula_groups"]) {
            item(
                output,
                format!(
                    "{} | {}",
                    value_text(&group["formula_name"]),
                    named(&group["group"], "All selected annotations")
                ),
            );
            body(
                output,
                format!(
                    "{} of {} annotations contributed | {} errors | {}",
                    value_text(&group["contributing_rows"]),
                    value_text(&group["matched_rows"]),
                    value_text(&group["error_rows"]),
                    if group["complete"] == true {
                        "complete calculation"
                    } else {
                        "partial calculation"
                    }
                ),
            );
            for total in entries(&group["totals"]) {
                body(
                    output,
                    format!(
                        "Total: {} {} ({} decimal places; {} annotations)",
                        value_text(&total["amount"]),
                        unit(&total["unit"]),
                        value_text(&total["rounding_decimals"]),
                        value_text(&total["contributing_rows"])
                    ),
                );
            }
            if entries(&group["totals"]).is_empty() {
                body(output, "Total: not available; no successful calculation.");
            }
            if let Some(errors) = group["errors"].as_object() {
                for (error, count) in errors {
                    body(
                        output,
                        format!(
                            "{}: {} annotations",
                            label(&Value::String(error.clone())),
                            value_text(count)
                        ),
                    );
                }
            }
        }
    }
}

fn annotation(output: &mut Vec<Block>, row: &Value, formulas: Option<&&Value>) {
    let register = &row["register"];
    item(
        output,
        format!(
            "Page {} | Annotation {} | {}",
            value_text(&register["page"]),
            value_text(&register["index"]),
            value_text(&register["kind"])
        ),
    );
    body(
        output,
        format!("Subject: {}", named(&register["subject"], "Not set")),
    );
    body(
        output,
        format!("Comment: {}", named(&register["contents"], "Not set")),
    );
    body(
        output,
        format!(
            "Status: {} | Review: {} | Discipline: {}",
            named(&register["fields"]["business_status"], "Not set"),
            named(&register["review_status"], "Not set"),
            named(&register["fields"]["discipline"], "Not set")
        ),
    );
    body(
        output,
        format!(
            "Author: {} | Modified: {}",
            named(&register["author"], "Not set"),
            named(&register["modified"], "Not set")
        ),
    );
    let zone = &register["zone"];
    let path = entries(&zone["path"])
        .iter()
        .map(|part| {
            format!(
                "{} [{}]",
                value_text(&part["name"]),
                value_text(&part["id"])
            )
        })
        .collect::<Vec<_>>()
        .join(" / ");
    body(
        output,
        format!(
            "Location: {}{}",
            label(&zone["status"]),
            if path.is_empty() {
                String::new()
            } else {
                format!(" | {path}")
            }
        ),
    );
    if !zone["zone_id"].is_null() {
        note(
            output,
            format!("Assigned zone ID: {}", value_text(&zone["zone_id"])),
        );
    }
    if !entries(&zone["candidates"]).is_empty() {
        note(
            output,
            format!(
                "Candidate zone IDs: {}",
                entries(&zone["candidates"])
                    .iter()
                    .map(value_text)
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
        );
    }
    body(output, format!("Quantity: {}", quantity(&row["quantity"])));
    if !row["measurement_validity"].is_null() {
        body(
            output,
            format!(
                "Measurement verification: {}",
                label(&row["measurement_validity"])
            ),
        );
    }
    if !row["historical_quantity"].is_null() {
        body(
            output,
            format!(
                "Historical quantity (excluded from totals): {}",
                quantity(&row["historical_quantity"])
            ),
        );
    }
    let price = &row["fields"]["unit_price"];
    if price.is_null() {
        body(output, "Unit price: not set.");
    } else {
        body(
            output,
            format!(
                "Unit price: {} {} per {} ({} decimal places)",
                value_text(&price["amount"]),
                value_text(&price["currency"]),
                unit(&price["per_unit"]),
                value_text(&price["rounding_decimals"])
            ),
        );
    }
    body(
        output,
        if row["line_total"].is_null() {
            "Line cost: not available; excluded from cost totals.".into()
        } else {
            format!("Line cost: {}", money(&row["line_total"]))
        },
    );
    if let Some(custom) = register["fields"]["custom"].as_object() {
        for (name, value) in custom {
            body(output, format!("{}: {}", visible(name), value_text(value)));
        }
    }
    if let Some(custom) = row["fields"]["custom"].as_object() {
        for (name, value) in custom {
            body(
                output,
                format!(
                    "{} ({}): {}{}",
                    visible(name),
                    label(&value["type"]),
                    value_text(&value["value"]),
                    if value["unit"].is_null() {
                        String::new()
                    } else {
                        format!(" {}", unit(&value["unit"]))
                    }
                ),
            );
            if let Some(options) = value["options"].as_array() {
                note(
                    output,
                    format!(
                        "Available choices: {}",
                        options
                            .iter()
                            .map(value_text)
                            .collect::<Vec<_>>()
                            .join("; ")
                    ),
                );
            }
        }
    }
    if let Some(values) = formulas.and_then(|v| v.as_object()) {
        for (name, value) in values {
            body(
                output,
                if !value["error"].is_null() {
                    format!(
                        "{}: not available ({})",
                        visible(name),
                        label(&value["error"])
                    )
                } else {
                    format!(
                        "{}: {} {}",
                        visible(name),
                        value_text(&value["value"]),
                        unit(&value["unit"])
                    )
                },
            );
        }
    }
    note(
        output,
        format!(
            "Annotation ID: {} | Count group: {}",
            named(&register["annotation_id"], "Not set"),
            named(&register["count_group"], "Not set")
        ),
    );
    note(
        output,
        format!(
            "Reply: {} | Parent index: {} | Hidden: {} | Locked: {} | Editable: {}",
            value_text(&register["is_reply"]),
            named_number(&register["parent"]),
            value_text(&register["hidden"]),
            value_text(&register["locked"]),
            value_text(&row["editable"])
        ),
    );
    note(
        output,
        format!(
            "Bounds in displayed page points: {}",
            if register["rect"].is_null() {
                "Unknown".into()
            } else {
                entries(&register["rect"])
                    .iter()
                    .map(value_text)
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        ),
    );
}

fn named_number(value: &Value) -> String {
    if value.is_null() {
        "Not set".into()
    } else {
        value_text(value)
    }
}

fn blocks(model: &IssueReportModel, cancel: &AtomicBool) -> Result<Vec<Block>, EngineError> {
    let mut output = Vec::new();
    styled(&mut output, model.title.clone(), TextStyle::Title, 0);
    note(
        &mut output,
        format!(
            "Source: {} | {}",
            model
                .provenance
                .saved_file_name
                .as_deref()
                .unwrap_or("Current document snapshot"),
            if model.selection.id() == "register" {
                "Annotation register selection"
            } else {
                "Quantity table selection"
            }
        ),
    );
    let rows = entries(&model.quantities["rows"]);
    summary(&mut output, model, rows);
    section(&mut output, "Issue groups");
    if model.issue_groups.is_empty() {
        body(&mut output, "No annotations match the selected filter.");
    }
    for group in &model.issue_groups {
        cancelled(cancel)?;
        item(
            &mut output,
            format!(
                "{} annotations | Status: {} | Review: {} | Discipline: {}",
                group.count,
                group.business_status.as_deref().unwrap_or("Not set"),
                group.review_status.as_deref().unwrap_or("Not set"),
                group.discipline.as_deref().unwrap_or("Not set")
            ),
        );
        note(
            &mut output,
            format!("Members: {}", identities(&group.members)),
        );
    }
    if model.legend_included {
        section(&mut output, "Legend of selected annotations");
        note(&mut output, "Samples show supported style declarations stored in the PDF at 1:5. Unknown appearances are not reconstructed.");
        if model.legend.is_empty() {
            body(&mut output, "No legend entries match the selected filter.");
        }
        for entry in &model.legend {
            cancelled(cancel)?;
            item(
                &mut output,
                format!(
                    "{} | {} | {} annotations{}",
                    entry.id,
                    entry.kind,
                    entry.count,
                    entry
                        .count_group
                        .as_ref()
                        .map_or(String::new(), |group| format!(" | {group}"))
                ),
            );
            if let Some(style) = &entry.style {
                output.push(Block {
                    text: "Declared style sample (1:5)".into(),
                    style: TextStyle::Body,
                    indent: 8,
                    sample: Some((entry.kind.clone(), style.clone())),
                    break_before: false,
                });
                note(&mut output, format!("Stroke RGB: {:?} | Fill RGB: {} | Width: {} pt | Opacity: {} | Dash: {:?} | Arrow: {}",
                    style.stroke_rgb, style.fill_rgb.map_or("None".into(), |color| format!("{color:?}")), style.width_pt,
                    style.opacity, style.dash_pattern, style.arrow));
            }
            body(
                &mut output,
                format!(
                    "Appearance: {}{}",
                    label(&Value::String(entry.style_status.into())),
                    entry
                        .reason
                        .as_ref()
                        .map_or(String::new(), |reason| format!("; {reason}"))
                ),
            );
            note(
                &mut output,
                format!("Members: {}", identities(&entry.members)),
            );
        }
    }
    section(&mut output, "Annotation details");
    let formulas = entries(&model.quantities["formula_rows"])
        .iter()
        .map(|value| {
            (
                (value["page"].as_u64(), value["index"].as_u64()),
                &value["values"],
            )
        })
        .collect::<BTreeMap<_, _>>();
    if rows.is_empty() {
        body(&mut output, "No annotations match the selected filter.");
    }
    for row in rows {
        cancelled(cancel)?;
        annotation(
            &mut output,
            row,
            formulas.get(&(
                row["register"]["page"].as_u64(),
                row["register"]["index"].as_u64(),
            )),
        );
    }
    section(&mut output, "Technical appendix");
    output.last_mut().expect("appendix heading").break_before = true;
    note(&mut output, "Audit information and exact calculation settings for this report. These settings do not change the source document.");
    section(&mut output, "Report provenance");
    flatten(
        &mut output,
        "",
        &serde_json::to_value(&model.provenance).expect("provenance"),
        0,
    )?;
    section(&mut output, "Selection and calculation settings");
    note(&mut output, "Zone coverage below describes the source page/reply scope before the remaining annotation filters. It is not the coverage count of the selected annotations.");
    let mut options = model.quantities.clone();
    for key in ["rows", "summary", "formula_rows"] {
        options
            .as_object_mut()
            .expect("quantity export")
            .remove(key);
    }
    flatten(&mut output, "", &options, 0)?;
    note(
        &mut output,
        format!(
            "Report format: {} / version {} | Legend included: {} | Checks complete: {}",
            model.schema, model.version, model.legend_included, model.complete
        ),
    );
    note(&mut output, format!("Summary reconciliation: {} matched annotations; {} excluded from additive quantities (including angles).",
        value_text(&model.quantities["summary"]["matched_rows"]), value_text(&model.quantities["summary"]["excluded_quantity_rows"])));
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
    let mut laid_out = Vec::<Vec<Line>>::with_capacity(blocks.len());
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
            let size = block.style.size();
            let indent = block.indent.min(8);
            let width = (WIDTH - 2.0 * MARGIN - indent as f32 * 9.0) * BODY / size;
            let mut lines = Vec::new();
            for (part, text) in wrapped(&mut probe, &block.text, width, cancel)?
                .into_iter()
                .enumerate()
            {
                let sample = if part == 0 {
                    block.sample.clone()
                } else {
                    None
                };
                lines.push(Line {
                    text,
                    style: block.style,
                    indent,
                    sample,
                    space_before: if part == 0 { block.style.gap() } else { 0.0 },
                });
            }
            laid_out.push(lines);
        }
    }
    let mut pages = vec![Vec::<Line>::new()];
    let mut used = 0.0;
    let available = HEIGHT - 2.0 * MARGIN - 28.0;
    for index in 0..laid_out.len() {
        cancelled(cancel)?;
        let full_height = |lines: &[Line]| {
            lines
                .iter()
                .map(|line| line.height() + line.space_before)
                .sum::<f32>()
        };
        let height = full_height(&laid_out[index]);
        // Keep short paragraphs intact, and keep a heading chain with readable content.
        let mut required = if height <= 140.0 || blocks[index].style.keep_with_next() {
            height
        } else {
            laid_out[index][0].height() + laid_out[index][0].space_before
        };
        if blocks[index].style.keep_with_next() {
            for next in index + 1..laid_out.len() {
                if blocks[next].break_before {
                    break;
                }
                let next_height = full_height(&laid_out[next]);
                required += if next_height <= 140.0 || blocks[next].style.keep_with_next() {
                    next_height
                } else {
                    laid_out[next][0].height() + laid_out[next][0].space_before
                };
                if !blocks[next].style.keep_with_next() {
                    break;
                }
            }
        }
        if used > 0.0 && (blocks[index].break_before || used + required > available) {
            pages.push(Vec::new());
            used = 0.0;
        }
        for mut line in std::mem::take(&mut laid_out[index]) {
            if used == 0.0 {
                line.space_before = 0.0;
            }
            if used + line.space_before + line.height() > available {
                pages.push(Vec::new());
                used = 0.0;
                line.space_before = 0.0;
            }
            if pages.len() > MAX_REPORT_PAGES {
                return Err(invalid("PDF exceeds 250 pages; narrow the filter"));
            }
            used += line.space_before + line.height();
            pages.last_mut().expect("page").push(line);
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
        let mut y = HEIGHT - MARGIN - 21.0;
        for line in lines {
            cancelled(cancel)?;
            y -= line.space_before;
            if !line.text.trim().is_empty() {
                let size = line.style.size();
                let mut object =
                    PdfPageTextObject::new(&pdf, &line.text, token, PdfPoints::new(size))?;
                object.set_fill_color(line.style.color())?;
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
            y -= line.height();
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
