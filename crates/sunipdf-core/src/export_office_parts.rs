use super::{archive::Package, cancelled, failure, xml_text, TextPage};
use crate::{Document, EngineError};
use std::sync::atomic::AtomicBool;

const REL: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const OFFICE: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const WORD: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const DRAW: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const PRESENT: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
const SHEET: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const GROUP: &str = "<p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/><a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"0\" cy=\"0\"/></a:xfrm></p:grpSpPr>";

fn part(package: &mut Package, name: &str, text: &str) -> Result<(), EngineError> {
    package.part(name, text.as_bytes())
}
fn relations(entries: &str) -> String {
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"{REL}\">{entries}</Relationships>")
}
fn relation(id: usize, kind: &str, target: &str) -> String {
    format!("<Relationship Id=\"rId{id}\" Type=\"{OFFICE}/{kind}\" Target=\"{target}\"/>")
}
fn content_types(entries: &str) -> String {
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Default Extension=\"png\" ContentType=\"image/png\"/>{entries}</Types>")
}
fn content(name: &str, kind: &str) -> String {
    format!("<Override PartName=\"/{name}\" ContentType=\"application/vnd.openxmlformats-officedocument.{kind}+xml\"/>")
}

fn page_properties(page: &TextPage, next: bool) -> String {
    format!("<w:sectPr>{}<w:pgSz w:w=\"{}\" w:h=\"{}\"/><w:pgMar w:top=\"720\" w:right=\"720\" w:bottom=\"720\" w:left=\"720\"/></w:sectPr>", if next { "<w:type w:val=\"nextPage\"/>" } else { "" }, (page.size.width_pt*20.0).round() as u32, (page.size.height_pt*20.0).round() as u32)
}

pub(super) fn word(pages: &[TextPage], package: &mut Package) -> Result<(), EngineError> {
    let mut xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><w:document xmlns:w=\"{WORD}\"><w:body>"
    );
    for (index, page) in pages.iter().enumerate() {
        for row in &page.rows {
            let mut text = String::new();
            for object in row {
                if !text.is_empty()
                    && !text.ends_with(char::is_whitespace)
                    && !object
                        .text
                        .as_deref()
                        .unwrap_or_default()
                        .starts_with(char::is_whitespace)
                {
                    text.push(' ');
                }
                text.push_str(object.text.as_deref().unwrap_or_default());
            }
            xml.push_str(&format!("<w:p><w:pPr><w:spacing w:before=\"0\" w:after=\"0\"/></w:pPr><w:r><w:rPr><w:sz w:val=\"22\"/></w:rPr><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",xml_text(&text)?));
        }
        if index + 1 < pages.len() {
            xml.push_str(&format!(
                "<w:p><w:pPr>{}</w:pPr></w:p>",
                page_properties(page, true)
            ));
        } else {
            xml.push_str(&page_properties(page, false));
        }
    }
    xml.push_str("</w:body></w:document>");
    part(package, "word/document.xml", &xml)?;
    part(
        package,
        "_rels/.rels",
        &relations(&relation(1, "officeDocument", "word/document.xml")),
    )?;
    part(
        package,
        "[Content_Types].xml",
        &content_types(&content(
            "word/document.xml",
            "wordprocessingml.document.main",
        )),
    )
}

fn column(mut index: usize) -> String {
    let mut chars = Vec::new();
    loop {
        chars.push(b'A' + (index % 26) as u8);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    chars.reverse();
    String::from_utf8(chars).expect("ASCII column name")
}

pub(super) fn workbook(pages: &[TextPage], package: &mut Package) -> Result<(), EngineError> {
    let mut workbook = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><workbook xmlns=\"{SHEET}\" xmlns:r=\"{OFFICE}\"><sheets>");
    let mut rels = String::new();
    let mut types = content("xl/workbook.xml", "spreadsheetml.sheet.main");
    for (index, page) in pages.iter().enumerate() {
        let number = index + 1;
        workbook.push_str(&format!(
            "<sheet name=\"Page {number}\" sheetId=\"{number}\" r:id=\"rId{number}\"/>"
        ));
        rels.push_str(&relation(
            number,
            "worksheet",
            &format!("worksheets/sheet{number}.xml"),
        ));
        types.push_str(&content(
            &format!("xl/worksheets/sheet{number}.xml"),
            "spreadsheetml.worksheet",
        ));
        let mut positions: Vec<f32> = page
            .rows
            .iter()
            .flat_map(|r| r.iter().map(|o| o.bounds.left))
            .collect();
        positions.sort_by(f32::total_cmp);
        let mut columns: Vec<f32> = Vec::new();
        for position in positions {
            if columns.last().is_none_or(|last| position - *last > 4.0) {
                columns.push(position);
            }
        }
        if columns.len() > 256 {
            return Err(failure("Excel export exceeds 256 inferred text columns"));
        }
        let mut xml = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><worksheet xmlns=\"{SHEET}\"><sheetData>"
        );
        for (row_index, row) in page.rows.iter().enumerate() {
            let row_number = row_index + 1;
            xml.push_str(&format!("<row r=\"{row_number}\">"));
            let mut previous = None;
            for object in row {
                let mut matching = columns
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| (object.bounds.left - **p).abs() <= 4.0);
                let col = matching
                    .next()
                    .map(|(index, _)| index)
                    .ok_or_else(|| failure("Excel text columns are ambiguous"))?;
                if matching.next().is_some() {
                    return Err(failure("Excel text columns are ambiguous"));
                }
                if previous == Some(col) {
                    return Err(failure(
                        "Excel export refuses multiple text objects in one inferred cell",
                    ));
                }
                previous = Some(col);
                xml.push_str(&format!("<c r=\"{}{row_number}\" t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>",column(col),xml_text(object.text.as_deref().unwrap_or_default())?));
            }
            xml.push_str("</row>");
        }
        xml.push_str("</sheetData></worksheet>");
        part(package, &format!("xl/worksheets/sheet{number}.xml"), &xml)?;
    }
    workbook.push_str("</sheets></workbook>");
    part(package, "xl/workbook.xml", &workbook)?;
    part(package, "xl/_rels/workbook.xml.rels", &relations(&rels))?;
    part(
        package,
        "_rels/.rels",
        &relations(&relation(1, "officeDocument", "xl/workbook.xml")),
    )?;
    part(package, "[Content_Types].xml", &content_types(&types))
}

fn theme() -> String {
    let mut colors = String::new();
    for (name, color) in [
        ("dk1", "000000"),
        ("lt1", "FFFFFF"),
        ("dk2", "222222"),
        ("lt2", "EEEEEE"),
        ("accent1", "4472C4"),
        ("accent2", "ED7D31"),
        ("accent3", "A5A5A5"),
        ("accent4", "FFC000"),
        ("accent5", "5B9BD5"),
        ("accent6", "70AD47"),
        ("hlink", "0000FF"),
        ("folHlink", "800080"),
    ] {
        colors.push_str(&format!(
            "<a:{name}><a:srgbClr val=\"{color}\"/></a:{name}>"
        ));
    }
    let fill = "<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>";
    let line=format!("<a:ln w=\"12700\" cap=\"flat\" cmpd=\"sng\" algn=\"ctr\">{fill}<a:prstDash val=\"solid\"/></a:ln>");
    let font = "<a:latin typeface=\"Arial\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/>";
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><a:theme xmlns:a=\"{DRAW}\" name=\"SuniPDF\"><a:themeElements><a:clrScheme name=\"Default\">{colors}</a:clrScheme><a:fontScheme name=\"Default\"><a:majorFont>{font}</a:majorFont><a:minorFont>{font}</a:minorFont></a:fontScheme><a:fmtScheme name=\"Default\"><a:fillStyleLst>{}</a:fillStyleLst><a:lnStyleLst>{}</a:lnStyleLst><a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst><a:bgFillStyleLst>{}</a:bgFillStyleLst></a:fmtScheme></a:themeElements><a:objectDefaults/><a:extraClrSchemeLst/></a:theme>",fill.repeat(3),line.repeat(3),fill.repeat(3))
}

pub(super) fn presentation(
    doc: &Document<'_>,
    package: &mut Package,
    cancel: &AtomicBool,
    progress: &mut impl FnMut(usize, usize),
) -> Result<(), EngineError> {
    let first = doc.page_size(0)?;
    let width = first.width_pt.clamp(72.0, 4032.0);
    let height = first.height_pt.clamp(72.0, 4032.0);
    let mut xml=format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><p:presentation xmlns:p=\"{PRESENT}\" xmlns:a=\"{DRAW}\" xmlns:r=\"{OFFICE}\"><p:sldMasterIdLst><p:sldMasterId id=\"2147483648\" r:id=\"rId1\"/></p:sldMasterIdLst><p:sldIdLst>");
    let mut rels = relation(1, "slideMaster", "slideMasters/slideMaster1.xml");
    let mut types = content("ppt/presentation.xml", "presentationml.presentation.main")
        + &content(
            "ppt/slideMasters/slideMaster1.xml",
            "presentationml.slideMaster",
        )
        + &content(
            "ppt/slideLayouts/slideLayout1.xml",
            "presentationml.slideLayout",
        )
        + &content("ppt/theme/theme1.xml", "theme");
    for page in 0..doc.page_count() {
        cancelled(cancel)?;
        let number = page + 1;
        let size = doc.page_size(page)?;
        let scale = 150.0 / 72.0;
        let pixels = (size.width_pt as f64 * scale as f64) * (size.height_pt as f64 * scale as f64);
        if !pixels.is_finite()
            || pixels > 8_000_000.0
            || size.width_pt * scale > 8192.0
            || size.height_pt * scale > 8192.0
        {
            return Err(EngineError::RenderTooLarge);
        }
        let rendered = doc.render_page(page, scale)?;
        let mut png = Vec::new();
        image::write_buffer_with_format(
            &mut std::io::Cursor::new(&mut png),
            &rendered.rgba,
            rendered.width,
            rendered.height,
            image::ColorType::Rgba8,
            image::ImageFormat::Png,
        )
        .map_err(|_| failure("Office page image encoding failed"))?;
        cancelled(cancel)?;
        package.part(&format!("ppt/media/page{number}.png"), &png)?;
        let fit = (width / size.width_pt).min(height / size.height_pt);
        let w = size.width_pt * fit;
        let h = size.height_pt * fit;
        let x = ((width - w) * 0.5 * 12700.0).round() as u32;
        let y = ((height - h) * 0.5 * 12700.0).round() as u32;
        let slide=format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><p:sld xmlns:p=\"{PRESENT}\" xmlns:a=\"{DRAW}\" xmlns:r=\"{OFFICE}\"><p:cSld><p:spTree>{GROUP}<p:pic><p:nvPicPr><p:cNvPr id=\"2\" name=\"PDF page {number}\"/><p:cNvPicPr><a:picLocks noChangeAspect=\"1\"/></p:cNvPicPr><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed=\"rId1\"/><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr><a:xfrm><a:off x=\"{x}\" y=\"{y}\"/><a:ext cx=\"{}\" cy=\"{}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr></p:pic></p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>",(w*12700.0).round() as u32,(h*12700.0).round() as u32);
        part(package, &format!("ppt/slides/slide{number}.xml"), &slide)?;
        part(
            package,
            &format!("ppt/slides/_rels/slide{number}.xml.rels"),
            &relations(
                &(relation(1, "image", &format!("../media/page{number}.png"))
                    + &relation(2, "slideLayout", "../slideLayouts/slideLayout1.xml")),
            ),
        )?;
        xml.push_str(&format!(
            "<p:sldId id=\"{}\" r:id=\"rId{}\"/>",
            256 + page,
            page + 2
        ));
        rels.push_str(&relation(
            page + 2,
            "slide",
            &format!("slides/slide{number}.xml"),
        ));
        types.push_str(&content(
            &format!("ppt/slides/slide{number}.xml"),
            "presentationml.slide",
        ));
        progress(page + 1, doc.page_count());
    }
    xml.push_str(&format!("</p:sldIdLst><p:sldSz cx=\"{}\" cy=\"{}\" type=\"custom\"/><p:notesSz cx=\"6858000\" cy=\"9144000\"/></p:presentation>",(width*12700.0).round() as u32,(height*12700.0).round() as u32));
    let master=format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><p:sldMaster xmlns:p=\"{PRESENT}\" xmlns:a=\"{DRAW}\" xmlns:r=\"{OFFICE}\"><p:cSld><p:spTree>{GROUP}</p:spTree></p:cSld><p:clrMap bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"accent1\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/><p:sldLayoutIdLst><p:sldLayoutId id=\"2147483649\" r:id=\"rId1\"/></p:sldLayoutIdLst></p:sldMaster>");
    let layout=format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><p:sldLayout xmlns:p=\"{PRESENT}\" xmlns:a=\"{DRAW}\" type=\"blank\" preserve=\"1\"><p:cSld><p:spTree>{GROUP}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>");
    for (name, value) in [
        ("ppt/presentation.xml", xml),
        ("ppt/_rels/presentation.xml.rels", relations(&rels)),
        ("ppt/slideMasters/slideMaster1.xml", master),
        (
            "ppt/slideMasters/_rels/slideMaster1.xml.rels",
            relations(
                &(relation(1, "slideLayout", "../slideLayouts/slideLayout1.xml")
                    + &relation(2, "theme", "../theme/theme1.xml")),
            ),
        ),
        ("ppt/slideLayouts/slideLayout1.xml", layout),
        (
            "ppt/slideLayouts/_rels/slideLayout1.xml.rels",
            relations(&relation(
                1,
                "slideMaster",
                "../slideMasters/slideMaster1.xml",
            )),
        ),
        ("ppt/theme/theme1.xml", theme()),
        (
            "_rels/.rels",
            relations(&relation(1, "officeDocument", "ppt/presentation.xml")),
        ),
        ("[Content_Types].xml", content_types(&types)),
    ] {
        part(package, name, &value)?;
    }
    Ok(())
}
