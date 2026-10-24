//! Deliberately formula-free OOXML. Exact decimal cells are retained as text beside numbers.

use crate::export_office::{archive::Package, xml_text};
use crate::quantity_workbook::{cancelled, invalid, QuantityWorkbookModel};
use crate::EngineError;
use serde_json::Value;
use std::sync::atomic::AtomicBool;

const SHEET: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const OFFICE: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const RELS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const MAX_XML_BYTES: usize = 48 * 1024 * 1024;

fn text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}
fn spreadsheet_text(value: &str) -> Result<String, EngineError> {
    // Spreadsheet strings have their own escape syntax in addition to XML entities.
    // Preserve literal escape-looking text and carriage returns through Excel readers.
    let mut encoded = String::with_capacity(value.len());
    for (index, ch) in value.char_indices() {
        let escape = ch == '_'
            && value.as_bytes().get(index + 1) == Some(&b'x')
            && value.as_bytes().get(index + 6) == Some(&b'_')
            && value
                .as_bytes()
                .get(index + 2..index + 6)
                .is_some_and(|digits| digits.iter().all(u8::is_ascii_hexdigit));
        if escape {
            encoded.push_str("_x005F_");
        } else if ch == '\r' {
            encoded.push_str("_x000D_");
        } else {
            encoded.push(ch);
        }
    }
    xml_text(&encoded)
}
enum Cell {
    Text(String),
    Number(String),
}
impl Cell {
    fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }
    fn value(value: &Value) -> Self {
        Self::Text(text(value))
    }
    fn numeric(value: &Value) -> Self {
        let raw = text(value);
        // Excel stores at most 15 significant decimal digits. Keep longer exact values as text.
        let significant = raw
            .bytes()
            .filter(u8::is_ascii_digit)
            .skip_while(|b| *b == b'0')
            .count();
        if !raw.is_empty() && significant <= 15 && raw.parse::<f64>().is_ok_and(f64::is_finite) {
            Self::Number(raw)
        } else {
            Self::Text(raw)
        }
    }
}
fn column(mut index: usize) -> String {
    let mut bytes = Vec::new();
    loop {
        bytes.push(b'A' + (index % 26) as u8);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    bytes.reverse();
    String::from_utf8(bytes).expect("ASCII column")
}
struct Worksheet {
    xml: String,
    row: usize,
    columns: usize,
}
impl Worksheet {
    fn new() -> Self {
        Self { xml: format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><worksheet xmlns=\"{SHEET}\"><sheetViews><sheetView workbookViewId=\"0\"><pane ySplit=\"1\" topLeftCell=\"A2\" activePane=\"bottomLeft\" state=\"frozen\"/></sheetView></sheetViews><sheetFormatPr defaultRowHeight=\"15\"/><sheetData>"), row: 0, columns: 0 }
    }
    fn row(&mut self, cells: Vec<Cell>) -> Result<(), EngineError> {
        self.row += 1;
        self.columns = self.columns.max(cells.len());
        if self.row > 50_000 || cells.len() > 256 {
            return Err(invalid("worksheet exceeds its row or column bounds"));
        }
        self.xml.push_str(&format!("<row r=\"{}\">", self.row));
        for (index, cell) in cells.into_iter().enumerate() {
            let reference = format!("{}{}", column(index), self.row);
            match cell {
                Cell::Text(value) => {
                    if value.chars().count() > 32767 {
                        return Err(invalid("worksheet text cell exceeds 32767 characters"));
                    }
                    let style = if self.row == 1 { " s=\"1\"" } else { "" };
                    self.xml.push_str(&format!("<c r=\"{reference}\"{style} t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>", spreadsheet_text(&value)?));
                }
                Cell::Number(value) => self.xml.push_str(&format!(
                    "<c r=\"{reference}\" t=\"n\"><v>{}</v></c>",
                    xml_text(&value)?
                )),
            }
        }
        self.xml.push_str("</row>");
        if self.xml.len() > MAX_XML_BYTES {
            return Err(invalid("worksheet exceeds 48 MiB"));
        }
        Ok(())
    }
    fn finish(mut self, filter: bool) -> String {
        self.xml.push_str("</sheetData>");
        if filter && self.row > 1 && self.columns > 0 {
            self.xml.push_str(&format!(
                "<autoFilter ref=\"A1:{}{}\"/>",
                column(self.columns - 1),
                self.row
            ));
        }
        self.xml.push_str("</worksheet>");
        self.xml
    }
}
fn headers(values: &[&str]) -> Vec<Cell> {
    values.iter().map(|value| Cell::text(*value)).collect()
}

pub(crate) fn render(
    model: &QuantityWorkbookModel,
    cancel: &AtomicBool,
) -> Result<Vec<u8>, EngineError> {
    let quantities = &model.quantities;
    let mut rows = Worksheet::new();
    let mut header = headers(&[
        "Annotation ID",
        "Page",
        "Index",
        "Kind",
        "Author",
        "Subject",
        "Contents",
        "Status",
        "Discipline",
        "Count group",
        "Quantity",
        "Unit",
        "Unit price",
        "Unit price exact",
        "Price unit",
        "Currency",
        "Line total",
        "Line total exact",
        "Measurement validity",
        "Zone status",
        "Zone path",
        "Custom fields",
    ]);
    let formulas = quantities["formulas"]
        .as_array()
        .ok_or_else(|| invalid("missing formula configuration"))?;
    for formula in formulas {
        let name = text(&formula["name"]);
        header.extend([
            Cell::text(format!("{name}: value")),
            Cell::text(format!("{name}: exact")),
            Cell::text(format!("{name}: unit")),
            Cell::text(format!("{name}: error")),
        ]);
    }
    rows.row(header)?;
    let computed: std::collections::HashMap<_, _> = quantities["formula_rows"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|row| {
            (
                (row["page"].as_u64(), row["index"].as_u64()),
                &row["values"],
            )
        })
        .collect();
    for row in quantities["rows"]
        .as_array()
        .ok_or_else(|| invalid("missing quantity rows"))?
    {
        cancelled(cancel)?;
        let r = &row["register"];
        let mut cells = vec![
            Cell::value(&r["annotation_id"]),
            Cell::numeric(&r["page"]),
            Cell::numeric(&r["index"]),
            Cell::value(&r["kind"]),
            Cell::value(&r["author"]),
            Cell::value(&r["subject"]),
            Cell::value(&r["contents"]),
            Cell::value(&r["fields"]["business_status"]),
            Cell::value(&r["fields"]["discipline"]),
            Cell::value(&r["count_group"]),
            Cell::numeric(&row["quantity"]["value"]),
            Cell::value(&row["quantity"]["unit"]),
            Cell::numeric(&row["fields"]["unit_price"]["amount"]),
            Cell::value(&row["fields"]["unit_price"]["amount"]),
            Cell::value(&row["fields"]["unit_price"]["per_unit"]),
            Cell::value(&row["fields"]["unit_price"]["currency"]),
            Cell::numeric(&row["line_total"]["amount"]),
            Cell::value(&row["line_total"]["amount"]),
            Cell::value(&row["measurement_validity"]),
            Cell::value(&r["zone"]["status"]),
            Cell::value(&r["zone"]["path"]),
            Cell::value(&row["fields"]["custom"]),
        ];
        for formula in formulas {
            let value = computed
                .get(&(r["page"].as_u64(), r["index"].as_u64()))
                .and_then(|values| values.get(formula["name"].as_str().unwrap_or("")))
                .unwrap_or(&Value::Null);
            cells.extend([
                Cell::numeric(&value["value"]),
                Cell::value(&value["value"]),
                Cell::value(&value["unit"]),
                Cell::value(&value["error"]),
            ]);
        }
        rows.row(cells)?;
    }
    let mut totals = Worksheet::new();
    totals.row(headers(&[
        "Record",
        "Group",
        "Formula",
        "Quantity",
        "Unit",
        "Amount",
        "Amount exact",
        "Currency",
        "Matched rows",
        "Contributing rows",
        "Missing or error rows",
        "Complete",
        "Details",
    ]))?;
    for group in quantities["summary"]["groups"]
        .as_array()
        .into_iter()
        .flatten()
    {
        cancelled(cancel)?;
        totals.row(vec![
            Cell::text("quantity"),
            Cell::value(&group["group"]),
            Cell::text(""),
            Cell::numeric(&group["quantity"]),
            Cell::value(&group["unit"]),
            Cell::text(""),
            Cell::text(""),
            Cell::text(""),
            Cell::numeric(&group["rows"]),
            Cell::numeric(&group["priced_rows"]),
            Cell::numeric(&group["missing_prices"]),
            Cell::text(""),
            Cell::text(""),
        ])?;
        for cost in group["costs"].as_array().into_iter().flatten() {
            totals.row(vec![
                Cell::text("cost"),
                Cell::value(&group["group"]),
                Cell::text(""),
                Cell::text(""),
                Cell::value(&group["unit"]),
                Cell::numeric(&cost["total"]["amount"]),
                Cell::value(&cost["total"]["amount"]),
                Cell::value(&cost["total"]["currency"]),
                Cell::numeric(&group["rows"]),
                Cell::numeric(&cost["priced_rows"]),
                Cell::numeric(&group["missing_prices"]),
                Cell::value(&cost["complete"]),
                Cell::text(""),
            ])?;
        }
    }
    for group in quantities["summary"]["formula_groups"]
        .as_array()
        .into_iter()
        .flatten()
    {
        cancelled(cancel)?;
        totals.row(vec![
            Cell::text("formula coverage"),
            Cell::value(&group["group"]),
            Cell::value(&group["formula_name"]),
            Cell::text(""),
            Cell::text(""),
            Cell::text(""),
            Cell::text(""),
            Cell::text(""),
            Cell::numeric(&group["matched_rows"]),
            Cell::numeric(&group["contributing_rows"]),
            Cell::numeric(&group["error_rows"]),
            Cell::value(&group["complete"]),
            Cell::value(&group["errors"]),
        ])?;
        for total in group["totals"].as_array().into_iter().flatten() {
            totals.row(vec![
                Cell::text("formula subtotal"),
                Cell::value(&group["group"]),
                Cell::value(&group["formula_name"]),
                Cell::text(""),
                Cell::value(&total["unit"]),
                Cell::numeric(&total["amount"]),
                Cell::value(&total["amount"]),
                Cell::text(""),
                Cell::numeric(&group["matched_rows"]),
                Cell::numeric(&total["contributing_rows"]),
                Cell::numeric(&group["error_rows"]),
                Cell::value(&group["complete"]),
                Cell::text(""),
            ])?;
        }
    }
    let mut changes = Worksheet::new();
    changes.row(headers(&[
        "Annotation ID",
        "Status",
        "Before page",
        "After page",
        "Before quantity",
        "Before unit",
        "After quantity",
        "After unit",
        "Before unit price exact",
        "After unit price exact",
        "Before line total exact",
        "After line total exact",
        "Before formulas",
        "After formulas",
        "Before comparison SHA256",
        "After comparison SHA256",
    ]))?;
    for change in &model.changes {
        cancelled(cancel)?;
        let value = serde_json::to_value(change).map_err(|e| invalid(&e.to_string()))?;
        let b = &value["before"]["row"];
        let a = &value["after"]["row"];
        changes.row(vec![
            Cell::value(&value["annotation_id"]),
            Cell::text(change.status.id()),
            Cell::numeric(&b["register"]["page"]),
            Cell::numeric(&a["register"]["page"]),
            Cell::numeric(&b["quantity"]["value"]),
            Cell::value(&b["quantity"]["unit"]),
            Cell::numeric(&a["quantity"]["value"]),
            Cell::value(&a["quantity"]["unit"]),
            Cell::value(&b["fields"]["unit_price"]["amount"]),
            Cell::value(&a["fields"]["unit_price"]["amount"]),
            Cell::value(&b["line_total"]["amount"]),
            Cell::value(&a["line_total"]["amount"]),
            Cell::value(&value["before"]["formulas"]),
            Cell::value(&value["after"]["formulas"]),
            Cell::value(&value["before"]["comparison_sha256"]),
            Cell::value(&value["after"]["comparison_sha256"]),
        ])?;
    }
    let mut info = Worksheet::new();
    info.row(headers(&["Property", "Value"]))?;
    for (key, value) in [
        ("Title", Value::String(model.title.clone())),
        ("Snapshot SHA256", Value::String(model.provenance.snapshot_sha256.clone())),
        ("Saved source SHA256", serde_json::to_value(&model.provenance.saved_file_sha256).expect("hash")),
        ("Previous snapshot SHA256", serde_json::to_value(&model.previous_snapshot_sha256).expect("hash")),
        ("Complete", Value::Bool(model.complete)),
        ("Refresh", Value::Bool(model.refresh)),
        ("New IDs included", Value::Bool(model.include_new)),
        ("Source encrypted", Value::Bool(model.provenance.source_encrypted)),
        ("Source signature count", Value::from(model.provenance.source_signature_count)),
        ("Output encrypted", Value::Bool(false)),
        ("Snapshot meaning", Value::String("Serialized document snapshot; distinct from the hash of the saved file bytes.".into())),
        ("Arithmetic", quantities["formula_arithmetic"].clone()),
        ("Pricing", quantities["money_rounding"].clone()),
        ("Zone coverage", quantities["zone_coverage"].clone()),
        ("Filter used for this export", quantities["filter"].clone()),
        ("Formula configuration", quantities["formulas"].clone()),
        ("Calculation policy", Value::String("Values calculated locally; no spreadsheet formulas, macros, external links or automatic refresh.".into())),
        ("Refresh policy", Value::String("Export a separate manifest and choose a current PDF to review a new workbook; this file and the PDF are never edited.".into())),
    ] { info.row(vec![Cell::text(key), Cell::value(&value)])?; }
    // Group and formula subtotals are structured rows in Totals. Repeating all
    // groups as one JSON cell would overflow Excel's cell limit on normal tables.
    for (key, value) in quantities["summary"]
        .as_object()
        .ok_or_else(|| invalid("missing quantity summary"))?
    {
        if key != "groups" && key != "formula_groups" {
            info.row(vec![
                Cell::text(format!("Quantity summary: {key}")),
                Cell::value(value),
            ])?;
        }
    }
    for warning in &model.warnings {
        info.row(vec![Cell::text("Warning"), Cell::text(warning)])?;
    }
    let sheets = [
        rows.finish(true),
        totals.finish(true),
        changes.finish(true),
        info.finish(false),
    ];
    let mut package = Package::default();
    let mut workbook = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><workbook xmlns=\"{SHEET}\" xmlns:r=\"{OFFICE}\"><sheets>");
    let mut relations =
        format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"{RELS}\">");
    let mut types = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/><Override PartName=\"/xl/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml\"/>".to_owned();
    for (index, (name, sheet)) in ["Quantities", "Totals", "Changes", "Provenance"]
        .iter()
        .zip(sheets)
        .enumerate()
    {
        cancelled(cancel)?;
        let n = index + 1;
        workbook.push_str(&format!(
            "<sheet name=\"{name}\" sheetId=\"{n}\" r:id=\"rId{n}\"/>"
        ));
        relations.push_str(&format!("<Relationship Id=\"rId{n}\" Type=\"{OFFICE}/worksheet\" Target=\"worksheets/sheet{n}.xml\"/>"));
        types.push_str(&format!("<Override PartName=\"/xl/worksheets/sheet{n}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>"));
        package.part(&format!("xl/worksheets/sheet{n}.xml"), sheet.as_bytes())?;
    }
    workbook.push_str("</sheets></workbook>");
    relations.push_str(&format!("<Relationship Id=\"rId5\" Type=\"{OFFICE}/styles\" Target=\"styles.xml\"/></Relationships>"));
    types.push_str("</Types>");
    let root_rels = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"{RELS}\"><Relationship Id=\"rId1\" Type=\"{OFFICE}/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>");
    let styles = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><styleSheet xmlns=\"{SHEET}\"><fonts count=\"2\"><font><sz val=\"11\"/><name val=\"Calibri\"/></font><font><b/><sz val=\"11\"/><name val=\"Calibri\"/></font></fonts><fills count=\"2\"><fill><patternFill patternType=\"none\"/></fill><fill><patternFill patternType=\"gray125\"/></fill></fills><borders count=\"1\"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/></cellStyleXfs><cellXfs count=\"2\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\"/><xf numFmtId=\"0\" fontId=\"1\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyFont=\"1\"/></cellXfs><cellStyles count=\"1\"><cellStyle name=\"Normal\" xfId=\"0\" builtinId=\"0\"/></cellStyles></styleSheet>");
    for (path, content) in [
        ("xl/workbook.xml", workbook),
        ("xl/_rels/workbook.xml.rels", relations),
        ("_rels/.rels", root_rels),
        ("[Content_Types].xml", types),
        ("xl/styles.xml", styles),
    ] {
        package.part(path, content.as_bytes())?;
    }
    cancelled(cancel)?;
    package.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantity_workbook::WorkbookProvenance;
    use serde_json::json;
    use std::collections::BTreeMap;
    use xml::reader::{EventReader, XmlEvent};

    fn parts(bytes: &[u8]) -> BTreeMap<String, String> {
        let mut result = BTreeMap::new();
        let mut offset = 0;
        while bytes.get(offset..offset + 4) == Some(b"PK\x03\x04") {
            let short = |i| u16::from_le_bytes(bytes[i..i + 2].try_into().unwrap()) as usize;
            let long = |i| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap()) as usize;
            assert_eq!(short(offset + 8), 0, "STORE package");
            let size = long(offset + 18);
            assert_eq!(size, long(offset + 22));
            let end = offset + 30 + short(offset + 26);
            let name = std::str::from_utf8(&bytes[offset + 30..end]).unwrap();
            let start = end + short(offset + 28);
            let xml = std::str::from_utf8(&bytes[start..start + size]).unwrap();
            for event in EventReader::from_str(xml) {
                event.unwrap();
            }
            assert!(result.insert(name.to_owned(), xml.to_owned()).is_none());
            offset = start + size;
        }
        assert_eq!(&bytes[offset..offset + 4], b"PK\x01\x02");
        result
    }

    fn cells(xml: &str) -> Vec<Vec<String>> {
        let mut rows = Vec::new();
        let mut row = Vec::new();
        let mut cell = None;
        for event in EventReader::from_str(xml) {
            match event.unwrap() {
                XmlEvent::StartElement { name, .. } if name.local_name == "c" => {
                    cell = Some(String::new());
                }
                XmlEvent::Characters(value) | XmlEvent::Whitespace(value) => {
                    if let Some(cell) = &mut cell {
                        cell.push_str(&value);
                    }
                }
                XmlEvent::EndElement { name } if name.local_name == "c" => {
                    row.push(cell.take().unwrap());
                }
                XmlEvent::EndElement { name } if name.local_name == "row" => {
                    rows.push(std::mem::take(&mut row));
                }
                _ => {}
            }
        }
        rows
    }

    #[test]
    fn hundreds_of_groups_keep_every_quantity_cost_and_formula_subtotal() {
        let count = 400;
        let mut rows = Vec::new();
        let mut groups = Vec::new();
        let mut formula_rows = Vec::new();
        let mut formula_groups = Vec::new();
        for index in 0..count {
            let group = format!("Group {index}");
            rows.push(json!({
                "register": {"annotation_id":format!("id-{index}"),"page":1,"index":index,
                    "kind":"Stamp","count_group":group,"fields":{},"zone":{"status":"unassigned"}},
                "quantity":{"value":1,"unit":"each"},
                "fields":{"unit_price":{"amount":"1.25","per_unit":"each","currency":"CHF"},"custom":{}},
                "line_total":{"amount":"1.25","currency":"CHF"}
            }));
            groups.push(json!({"group":group,"unit":"each","quantity":1,"rows":1,
                "priced_rows":1,"missing_prices":0,"costs":[{"total":{"amount":"1.25","currency":"CHF"},"priced_rows":1,"complete":true}]}));
            formula_rows.push(json!({"page":1,"index":index,"values":{"Double":{"value":"2","unit":"each","error":null}}}));
            formula_groups.push(
                json!({"group":group,"formula_name":"Double","matched_rows":1,
                "contributing_rows":1,"error_rows":0,"errors":{},"complete":true,
                "totals":[{"amount":"2","unit":"each","contributing_rows":1}]}),
            );
        }
        let summary = json!({"matched_rows":count,"quantity_rows":count,"additive_quantity_rows":count,
            "angular_rows":0,"unquantified_rows":0,"excluded_quantity_rows":0,
            "unverified_measurement_rows":0,"legacy_measurement_rows":0,"stale_measurement_rows":0,
            "groups":groups,"formula_groups":formula_groups});
        assert!(
            summary.to_string().len() > 32767,
            "regression exceeds a single cell"
        );
        let model = QuantityWorkbookModel {
            schema: "sunipdf-quantity-workbook",
            version: 1,
            title: "Many groups".into(),
            provenance: WorkbookProvenance {
                snapshot_sha256: "a".repeat(64),
                saved_file_sha256: None,
                source_encrypted: false,
                source_signature_count: 0,
            },
            quantities: json!({"rows":rows,"summary":summary,
                "formulas":[{"name":"Double","expression":"quantity * 2"}],
                "formula_rows":formula_rows,"filter":{},"zone_coverage":{"complete":true,"counts":{}}}),
            changes: Vec::new(),
            refresh: false,
            include_new: false,
            previous_snapshot_sha256: None,
            complete: true,
            warnings: Vec::new(),
            refreshable_rows: count,
        };
        let bytes = render(&model, &AtomicBool::new(false)).unwrap();
        let archive = parts(&bytes);
        assert_eq!(archive.len(), 9);
        assert!(archive
            .values()
            .all(|xml| !xml.contains("<f>") && !xml.contains("TargetMode=\"External\"")));
        assert_eq!(cells(&archive["xl/worksheets/sheet1.xml"]).len(), count + 1);
        let totals = cells(&archive["xl/worksheets/sheet2.xml"]);
        assert_eq!(totals.len(), count * 4 + 1);
        for index in 0..count {
            let group = format!("Group {index}");
            let matching: BTreeMap<_, _> = totals
                .iter()
                .skip(1)
                .filter(|row| row[1] == group)
                .map(|row| (row[0].as_str(), row))
                .collect();
            assert_eq!(matching.len(), 4);
            let quantity = matching["quantity"];
            assert_eq!((quantity[3].as_str(), quantity[4].as_str()), ("1", "each"));
            let cost = matching["cost"];
            assert_eq!(
                (cost[5].as_str(), cost[6].as_str(), cost[7].as_str()),
                ("1.25", "1.25", "CHF")
            );
            assert_eq!(
                (cost[9].as_str(), cost[10].as_str(), cost[11].as_str()),
                ("1", "0", "true")
            );
            let formula = matching["formula subtotal"];
            assert_eq!(
                (
                    formula[2].as_str(),
                    formula[4].as_str(),
                    formula[5].as_str(),
                    formula[6].as_str()
                ),
                ("Double", "each", "2", "2")
            );
            assert_eq!(matching["formula coverage"][11], "true");
        }
        let info = cells(&archive["xl/worksheets/sheet4.xml"]);
        assert!(info
            .iter()
            .any(|row| row[0] == "Quantity summary: matched_rows" && row[1] == "400"));
        assert!(info
            .iter()
            .all(|row| row.iter().all(|value| value.chars().count() <= 32767)));
        assert!(matches!(
            render(&model, &AtomicBool::new(true)),
            Err(EngineError::Cancelled)
        ));
    }

    #[test]
    fn spreadsheet_escape_looking_text_and_carriage_returns_are_preserved() {
        let original = "=SUM(A1) <&> _x0041_ _x005F_\r\n\t日本語";
        let mut sheet = Worksheet::new();
        sheet.row(vec![Cell::text(original)]).unwrap();
        let xml = sheet.finish(false);
        assert!(!xml.contains("<f>"));
        assert!(xml.contains("t=\"inlineStr\""));
        // XML parsing must retain the ST_Xstring escapes for the spreadsheet reader.
        assert_eq!(
            cells(&xml)[0][0],
            "=SUM(A1) <&> _x005F_x0041_ _x005F_x005F__x000D_\n\t日本語"
        );
        assert_eq!(
            spreadsheet_text("_x0041__x000D_").unwrap(),
            "_x005F_x0041__x005F_x000D_"
        );
        assert_eq!(spreadsheet_text("_x123Z_").unwrap(), "_x123Z_");
        assert!(spreadsheet_text("bad\0text").is_err());
    }
}
