//! Opt-in measured exports of pinned government publications; no customer documents or claimed accuracy.

use lopdf::{Dictionary, Object, ObjectId};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
};
use sunipdf_core::{
    export_office::Format,
    office::{Converter, Profile},
    Engine, ObjectKind,
};

fn sha256(path: &Path) -> String {
    let output=Command::new("powershell.exe").args(["-NoProfile","-NonInteractive","-Command","$s=[IO.File]::OpenRead($env:SUNIPDF_OFFICE_HASH_FILE);$h=[Security.Cryptography.SHA256]::Create();try{[BitConverter]::ToString($h.ComputeHash($s)).Replace('-','')}finally{$s.Dispose();$h.Dispose()}"]).env("SUNIPDF_OFFICE_HASH_FILE",path).output().unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().into()
}
fn parts(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut offset = 0;
    let mut result = BTreeMap::new();
    while bytes.get(offset..offset + 4) == Some(b"PK\x03\x04") {
        let short = |i| u16::from_le_bytes(bytes[i..i + 2].try_into().unwrap()) as usize;
        let len = u32::from_le_bytes(bytes[offset + 18..offset + 22].try_into().unwrap()) as usize;
        assert_eq!(short(offset + 8), 0);
        let name_len = short(offset + 26);
        let start = offset + 30 + name_len + short(offset + 28);
        result.insert(
            String::from_utf8(bytes[offset + 30..offset + 30 + name_len].to_vec()).unwrap(),
            bytes[start..start + len].to_vec(),
        );
        offset = start + len;
    }
    result
}
fn exported_text(parts: &BTreeMap<String, Vec<u8>>) -> String {
    let mut text = String::new();
    for (name, bytes) in parts {
        if name == "word/document.xml" || name.starts_with("xl/worksheets/sheet") {
            let mut inside = false;
            for event in xml::reader::EventReader::new(bytes.as_slice()) {
                match event.unwrap() {
                    xml::reader::XmlEvent::StartElement { name, .. } => {
                        inside = name.local_name == "t";
                    }
                    xml::reader::XmlEvent::Characters(value)
                    | xml::reader::XmlEvent::Whitespace(value)
                        if inside =>
                    {
                        text.push_str(&value)
                    }
                    xml::reader::XmlEvent::EndElement { .. } => inside = false,
                    _ => {}
                }
            }
        }
    }
    text
}
fn compact(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

#[derive(Debug, PartialEq)]
struct LogicalSnapshot {
    encrypted: bool,
    encryption: Option<Dictionary>,
    objects: BTreeMap<ObjectId, Object>,
    trailer: Dictionary,
}

fn logical_snapshot(bytes: &[u8]) -> LogicalSnapshot {
    let mut pdf = lopdf::Document::load_mem(bytes).unwrap();
    let encrypted = pdf.is_encrypted();
    let encryption = pdf.get_encrypted().ok().cloned();
    assert!(
        !encrypted,
        "controlled Office fixture must remain unencrypted"
    );
    let mut ids = pdf.trailer.get(b"ID").unwrap().as_array().unwrap().clone();
    assert_eq!(ids.len(), 2);
    assert_eq!(ids[0].as_str().unwrap(), b"SuniPDFOfficeQA!");
    let Object::String(_, format) = &ids[1] else {
        panic!("second trailer ID is not a string");
    };
    ids[1] = Object::String(Vec::new(), *format);
    pdf.trailer.set("ID", ids);
    LogicalSnapshot {
        encrypted,
        encryption,
        objects: pdf.objects,
        trailer: pdf.trailer,
    }
}

#[test]
#[ignore = "requires pinned scripts/fetch-regression-corpus.ps1 downloads; records actual licensed export fidelity"]
fn measure_licensed_office_exports_and_installed_converter_roundtrips() {
    let corpus = PathBuf::from(
        std::env::var_os("SUNIPDF_REAL_CORPUS").expect("fetch the pinned public corpus first"),
    );
    let report_path = PathBuf::from(
        std::env::var_os("SUNIPDF_OFFICE_REPORT").expect("set an ignored JSON report path"),
    );
    assert!(
        !report_path.exists(),
        "measurement reports must be new copies"
    );
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../../scripts/regression-corpus.json")).unwrap();
    let engine = Engine::new().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cancel = AtomicBool::new(false);
    let converter = Converter::detect();
    let required_converter = std::env::var("GITHUB_ACTIONS").is_ok_and(|value| value == "true")
        && std::env::var("SUNIPDF_REQUIRE_OFFICE_CONVERTER").is_ok_and(|value| value == "1");
    if required_converter {
        assert!(
            converter.is_some(),
            "pinned converter QA was requested but the installed converter is unavailable"
        );
    }
    let mut records = Vec::new();
    let mut roundtrips = 0;
    let mut verified_formats = std::collections::BTreeSet::new();
    for source in manifest["documents"].as_array().unwrap() {
        let id = source["id"].as_str().unwrap();
        let path = corpus.join(source["filename"].as_str().unwrap());
        assert_eq!(sha256(&path), source["sha256"].as_str().unwrap());
        assert_eq!(
            fs::metadata(&path).unwrap().len(),
            source["bytes"].as_u64().unwrap()
        );
        let original = fs::read(&path).unwrap();
        let doc = engine.open(&path, None).unwrap();
        assert_eq!(
            doc.page_count(),
            source["page_count"].as_u64().unwrap() as usize
        );
        let source_page = source["sample_page"].as_u64().unwrap() as usize;
        let page = source_page
            .checked_sub(1)
            .expect("manifest sample pages are one-based");
        let sample_path = dir.path().join(format!("{id}-sample.pdf"));
        doc.extract_to(&[page], &sample_path).unwrap();
        let sample = engine.open(&sample_path, None).unwrap();
        let text = sample.page_text(0).unwrap();
        let objects = sample.page_objects(0).unwrap();
        for format in [Format::Word, Format::Excel, Format::PowerPoint] {
            let output = dir.path().join(format!("{id}.{}", format.extension()));
            let mut record = serde_json::json!({"source_id":id,"source_sha256":source["sha256"],"license_evidence":source["license_evidence"],"sample_page_one_based":source_page,"sample_page_zero_based":page,"source_sample_sha256":sha256(&sample_path),"format":format.id(),"source_text_chars":text.chars().count(),"source_image_objects":objects.iter().filter(|o|o.kind==ObjectKind::Image).count(),"source_path_objects":objects.iter().filter(|o|o.kind==ObjectKind::Path).count(),"source_annotation_count":sample.annotations(0).unwrap().len(),"semantic_table_reconstruction":"not supported","original_unchanged":true});
            match sample.export_office(&output, format, &cancel, |_, _| {}) {
                Ok(report) => {
                    let package = parts(&fs::read(&output).unwrap());
                    record["status"] = "exported".into();
                    record["report"] = report.json();
                    record["output_sha256"] = sha256(&output).into();
                    if format == Format::PowerPoint {
                        let image = image::load_from_memory(&package["ppt/media/page1.png"])
                            .unwrap()
                            .into_rgba8();
                        let expected = sample.render_page(0, 150.0 / 72.0).unwrap();
                        assert_eq!(image.as_raw(), &expected.rgba);
                        assert_eq!(
                            (image.width(), image.height()),
                            (expected.width, expected.height)
                        );
                        record["page_image_pixels"] =
                            serde_json::json!(u64::from(image.width()) * u64::from(image.height()));
                        record["page_image_pixel_equality"] = true.into();
                        record["editable_text_reconstruction"] = false.into();
                    } else {
                        let actual = exported_text(&package);
                        record["exported_text_chars"] = actual.chars().count().into();
                        record["nonwhitespace_text_sequence_equal_to_pdfium"] =
                            (compact(&actual) == compact(&text)).into();
                        record["original_font_and_layout_reconstruction"] = false.into();
                    }
                    if let Some(converter) = &converter {
                        let mut profiles = Vec::new();
                        for profile in [Profile::Standard, Profile::Print] {
                            let pdf = dir.path().join(format!(
                                "{id}-{}-{}-roundtrip.pdf",
                                format.id(),
                                profile.id()
                            ));
                            let conversion = sunipdf_core::office::convert(
                                &engine, converter, &output, &pdf, profile, &cancel,
                            )
                            .unwrap();
                            let reopened = engine.open(&pdf, None).unwrap();
                            let mut roundtrip_text = String::new();
                            for page in 0..reopened.page_count() {
                                roundtrip_text.push_str(&reopened.page_text(page).unwrap());
                            }
                            let expected = sample.render_page(0, 150.0 / 72.0).unwrap();
                            let actual = reopened.render_page(0, 150.0 / 72.0).unwrap();
                            let same_dimensions =
                                (expected.width, expected.height) == (actual.width, actual.height);
                            let pixel_error = if same_dimensions {
                                Some(
                                    expected
                                        .rgba
                                        .iter()
                                        .zip(&actual.rgba)
                                        .map(|(a, b)| u64::from(a.abs_diff(*b)))
                                        .sum::<u64>() as f64
                                        / expected.rgba.len() as f64,
                                )
                            } else {
                                None
                            };
                            profiles.push(serde_json::json!({"conversion":conversion.json(),"output_sha256":sha256(&pdf),"nonwhitespace_text_sequence_equal_to_source":compact(&roundtrip_text)==compact(&text),"source_sample_page_size_pt":[sample.page_size(0).unwrap().width_pt,sample.page_size(0).unwrap().height_pt],"roundtrip_first_page_size_pt":[reopened.page_size(0).unwrap().width_pt,reopened.page_size(0).unwrap().height_pt],"first_page_render_dimensions_equal":same_dimensions,"first_page_mean_absolute_rgba_error_0_to_255":pixel_error,"semantic_table_and_font_reconstruction":"not supported; no inferred fidelity score"}));
                            roundtrips += 1;
                            verified_formats.insert((format.id(), profile.id()));
                        }
                        record["local_roundtrips"] = profiles.into();
                    } else {
                        record["local_roundtrip_status"] =
                            "unavailable: no installed supported converter tested".into();
                    }
                }
                Err(error) => {
                    assert!(!output.exists());
                    record["status"] = "refused".into();
                    record["reason"] = error.to_string().into();
                }
            }
            records.push(record);
        }
        assert_eq!(fs::read(&path).unwrap(), original);
    }
    // Controlled inputs prove converter coverage separately from complex publication refusals.
    let controlled_path = dir.path().join("controlled-simple-text.pdf");
    let mut controlled_input =
        lopdf::Document::load_mem(include_bytes!("fixtures/two-pages.pdf")).unwrap();
    controlled_input.trailer.set(
        "ID",
        vec![
            Object::string_literal(b"SuniPDFOfficeQA!".to_vec()),
            Object::string_literal(b"SuniPDFOfficeQA!".to_vec()),
        ],
    );
    let mut controlled_seed = Vec::new();
    controlled_input.save_to(&mut controlled_seed).unwrap();
    let mut controlled = engine
        .open_bytes(controlled_seed, None, &controlled_path)
        .unwrap();
    controlled.delete_page(1).unwrap();
    let indices: Vec<_> = controlled
        .page_objects(0)
        .unwrap()
        .iter()
        .map(|object| object.index)
        .collect();
    controlled.delete_objects(0, &indices).unwrap();
    for (line, text) in [
        "SuniPDF local Office conversion",
        "Literal string =1+1",
        "Simple controlled text fixture",
    ]
    .iter()
    .enumerate()
    {
        controlled
            .add_text(
                0,
                72.0,
                700.0 - line as f32 * 24.0,
                text,
                &sunipdf_core::TextStyle::default(),
            )
            .unwrap();
    }
    let controlled_bytes = controlled.snapshot().unwrap();
    let controlled_before = logical_snapshot(&controlled_bytes);
    sunipdf_core::write_new_copy(&controlled_path, &controlled_bytes).unwrap();
    let controlled_text = controlled.page_text(0).unwrap();
    let expected = controlled.render_page(0, 150.0 / 72.0).unwrap();
    let mut controlled_records = Vec::new();
    for format in [Format::Word, Format::Excel, Format::PowerPoint] {
        let output = dir
            .path()
            .join(format!("controlled-simple-text.{}", format.extension()));
        let exported = controlled
            .export_office(&output, format, &cancel, |_, _| {})
            .expect("controlled simple-text export must be supported");
        let package = parts(&fs::read(&output).unwrap());
        let mut record = serde_json::json!({"source_kind":"controlled synthetic repository fixture; not a NIST sample or customer document","source_sha256":sha256(&controlled_path),"format":format.id(),"source_text_chars":controlled_text.chars().count(),"status":"exported","report":exported.json(),"original_unchanged":true,"output_sha256":sha256(&output),"semantic_table_and_original_font_reconstruction":"not supported"});
        if format == Format::PowerPoint {
            let image = image::load_from_memory(&package["ppt/media/page1.png"])
                .unwrap()
                .into_rgba8();
            assert_eq!(image.as_raw(), &expected.rgba);
            record["page_image_pixel_equality"] = true.into();
        } else {
            let text = exported_text(&package);
            assert_eq!(compact(&text), compact(&controlled_text));
            record["exported_text_chars"] = text.chars().count().into();
            record["nonwhitespace_text_sequence_equal_to_source"] = true.into();
        }
        if let Some(converter) = &converter {
            let mut profiles = Vec::new();
            for profile in [Profile::Standard, Profile::Print] {
                let pdf =
                    dir.path()
                        .join(format!("controlled-{}-{}.pdf", format.id(), profile.id()));
                let converted = sunipdf_core::office::convert(
                    &engine, converter, &output, &pdf, profile, &cancel,
                )
                .unwrap();
                let reopened = engine.open(&pdf, None).unwrap();
                let mut text = String::new();
                for page in 0..reopened.page_count() {
                    text.push_str(&reopened.page_text(page).unwrap());
                }
                let rendered = reopened.render_page(0, 150.0 / 72.0).unwrap();
                let same = (expected.width, expected.height) == (rendered.width, rendered.height);
                let error = if same {
                    Some(
                        expected
                            .rgba
                            .iter()
                            .zip(&rendered.rgba)
                            .map(|(a, b)| u64::from(a.abs_diff(*b)))
                            .sum::<u64>() as f64
                            / expected.rgba.len() as f64,
                    )
                } else {
                    None
                };
                profiles.push(serde_json::json!({"conversion":converted.json(),"output_sha256":sha256(&pdf),"nonwhitespace_text_sequence_equal_to_source":compact(&text)==compact(&controlled_text),"source_size_pt":[controlled.page_size(0).unwrap().width_pt,controlled.page_size(0).unwrap().height_pt],"first_page_size_pt":[reopened.page_size(0).unwrap().width_pt,reopened.page_size(0).unwrap().height_pt],"first_page_render_dimensions_equal":same,"first_page_mean_absolute_rgba_error_0_to_255":error,"layout_fidelity":"measured only; original layout not guaranteed for editable exports"}));
                roundtrips += 1;
                verified_formats.insert((format.id(), profile.id()));
            }
            record["local_roundtrips"] = profiles.into();
        } else {
            record["local_roundtrip_status"] = "unavailable: no local converter tested".into();
        }
        controlled_records.push(record);
    }
    assert_eq!(
        logical_snapshot(&controlled.snapshot().unwrap()),
        controlled_before
    );
    assert_eq!(fs::read(&controlled_path).unwrap(), controlled_bytes);
    let revision = std::env::var("GITHUB_SHA").unwrap_or_else(|_| "unavailable outside CI".into());
    if required_converter {
        for format in [Format::Word, Format::Excel, Format::PowerPoint] {
            for profile in [Profile::Standard, Profile::Print] {
                assert!(
                    verified_formats.contains(&(format.id(), profile.id())),
                    "pinned converter QA requires a real {} / {} roundtrip",
                    format.id(),
                    profile.id()
                );
            }
        }
    }
    let report = serde_json::json!({"schema":2,"measurement":"actual pinned public source samples and separately labeled controlled fixtures; no inferred accuracy percentages","source_revision":revision,"build_version":env!("CARGO_PKG_VERSION"),"license_policy":manifest["license_policy_url"],"source_documents":4,"source_pages":744,"converter_detected":converter.is_some(),"completed_local_roundtrips":roundtrips,"records":records,"controlled_fixture_records":controlled_records});
    sunipdf_core::write_new_copy(&report_path, &serde_json::to_vec_pretty(&report).unwrap())
        .unwrap();
    println!(
        "Office fidelity measurements written to {}",
        report_path.display()
    );
}
