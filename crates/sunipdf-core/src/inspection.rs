//! Read-only inventory of stored PDF dictionaries, including unreachable objects.

use crate::{security, EngineError};
use lopdf::{Dictionary, Object};
use sha2::{Digest, Sha256};
use std::{io::Read, path::Path};

pub const MAX_INPUT_BYTES: usize = 128 * 1024 * 1024;
const MAX_OBJECTS: usize = 250_000;
const MAX_NODES: usize = 1_000_000;
const MAX_DEPTH: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub kind: &'static str,
    /// Matching dictionaries, not unique files, scripts, actions or signatures.
    pub dictionaries: usize,
}

#[derive(Debug, Clone)]
pub struct Inspection {
    pub source_sha256: String,
    pub source_bytes: usize,
    pub stored_objects: usize,
    pub examined_nodes: usize,
    pub complete: bool,
    pub limitations: Vec<&'static str>,
    pub findings: Vec<Finding>,
}

impl Inspection {
    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "source_sha256": self.source_sha256, "source_bytes": self.source_bytes,
            "stored_objects": self.stored_objects, "examined_nodes": self.examined_nodes,
            "complete": self.complete, "limitations": self.limitations,
            "findings": self.findings.iter().map(|finding| serde_json::json!({
                "kind": finding.kind, "dictionaries": finding.dictionaries,
            })).collect::<Vec<_>>(),
            "scope": "stored PDF dictionaries, including unreachable objects",
            "stream_contents_examined": false, "malware_scan": false,
            "signature_integrity_verified": false, "conformance_validated": false,
            "network_accessed": false, "file_modified": false,
        })
    }

    fn incomplete(&mut self, reason: &'static str) {
        self.complete = false;
        if !self.limitations.contains(&reason) {
            self.limitations.push(reason);
        }
    }

    fn name<'a>(
        &mut self,
        pdf: &'a lopdf::Document,
        dict: &'a Dictionary,
        key: &[u8],
    ) -> Option<&'a [u8]> {
        let value = dict.get(key).ok()?;
        match pdf
            .dereference(value)
            .and_then(|(_, value)| value.as_name())
        {
            Ok(name) => Some(name),
            Err(_) => {
                self.incomplete("unresolved_or_malformed_feature_name");
                None
            }
        }
    }

    fn dictionary(&mut self, pdf: &lopdf::Document, dict: &Dictionary) {
        let action = self.name(pdf, dict, b"S");
        let object_type = self.name(pdf, dict, b"Type");
        let subtype = self.name(pdf, dict, b"Subtype");
        let matches = [
            dict.has(b"JS") || dict.has(b"JavaScript") || action == Some(b"JavaScript"),
            dict.has(b"OpenAction") || dict.has(b"AA"),
            matches!(
                action,
                Some(b"URI" | b"Launch" | b"GoToR" | b"GoToE" | b"SubmitForm" | b"ImportData")
            ),
            dict.has(b"EmbeddedFiles")
                || dict.has(b"EF")
                || dict.has(b"AF")
                || object_type == Some(b"EmbeddedFile"),
            dict.has(b"XFA"),
            matches!(
                subtype,
                Some(b"RichMedia" | b"Movie" | b"Sound" | b"3D" | b"Screen")
            ),
            object_type == Some(b"Sig") || self.name(pdf, dict, b"FT") == Some(b"Sig"),
        ];
        for (finding, matched) in self.findings.iter_mut().zip(matches) {
            finding.dictionaries += usize::from(matched);
        }
    }

    fn visit(&mut self, pdf: &lopdf::Document, object: &Object, depth: usize) {
        if self.examined_nodes >= MAX_NODES || depth > MAX_DEPTH {
            self.incomplete("dictionary_traversal_limit");
            return;
        }
        self.examined_nodes += 1;
        match object {
            Object::Reference(id) => {
                if !pdf.objects.contains_key(id) {
                    self.incomplete("unresolved_object_reference");
                }
                // Stored objects are visited once below; following references would count cycles twice.
            }
            Object::Array(values) => {
                for value in values {
                    if self.examined_nodes >= MAX_NODES {
                        self.incomplete("dictionary_traversal_limit");
                        break;
                    }
                    self.visit(pdf, value, depth + 1);
                }
            }
            Object::Dictionary(dict) => self.visit_dictionary(pdf, dict, depth),
            Object::Stream(stream) => self.visit_dictionary(pdf, &stream.dict, depth),
            _ => {}
        }
    }

    fn visit_dictionary(&mut self, pdf: &lopdf::Document, dict: &Dictionary, depth: usize) {
        self.dictionary(pdf, dict);
        for (_, value) in dict {
            if self.examined_nodes >= MAX_NODES {
                self.incomplete("dictionary_traversal_limit");
                break;
            }
            self.visit(pdf, value, depth + 1);
        }
    }
}

/// Inventory the supplied revision without rendering or executing actions; payloads are not inspected.
pub fn inspect(input: &[u8], password: Option<&str>) -> Result<Inspection, EngineError> {
    if input.len() > MAX_INPUT_BYTES {
        return Err(EngineError::Other(
            "PDF inspection input exceeds 128 MiB".into(),
        ));
    }
    let pdf = security::load(input, password)?;
    if pdf.objects.len() > MAX_OBJECTS {
        return Err(EngineError::Other(
            "PDF inspection object limit exceeded".into(),
        ));
    }
    let mut report = Inspection {
        source_sha256: Sha256::digest(input)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        source_bytes: input.len(),
        stored_objects: pdf.objects.len(),
        examined_nodes: 0,
        complete: true,
        limitations: Vec::new(),
        findings: [
            "javascript",
            "automatic_actions",
            "external_actions",
            "embedded_files",
            "xfa",
            "multimedia",
            "signatures",
        ]
        .into_iter()
        .map(|kind| Finding {
            kind,
            dictionaries: 0,
        })
        .collect(),
    };
    report.visit_dictionary(&pdf, &pdf.trailer, 0);
    for object in pdf.objects.values() {
        if report.examined_nodes >= MAX_NODES {
            report.incomplete("dictionary_traversal_limit");
            break;
        }
        report.visit(&pdf, object, 0);
    }
    Ok(report)
}

/// The cap also applies while reading, even when a file grows after opening.
pub fn inspect_file(path: &Path, password: Option<&str>) -> Result<Inspection, EngineError> {
    let file = std::fs::File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(EngineError::Other(
            "Choose a regular PDF file for inspection".into(),
        ));
    }
    if metadata.len() > MAX_INPUT_BYTES as u64 {
        return Err(EngineError::Other(
            "PDF inspection input exceeds 128 MiB".into(),
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAX_INPUT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    inspect(&bytes, password)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{dictionary, Stream};

    fn fixture() -> lopdf::Document {
        let mut pdf = lopdf::Document::with_version("1.7");
        let pages =
            pdf.add_object(dictionary! {"Type"=>"Pages", "Kids"=>Vec::<Object>::new(), "Count"=>0});
        let root = pdf.add_object(dictionary! {"Type"=>"Catalog", "Pages"=>pages});
        pdf.trailer.set("Root", root);
        pdf
    }
    fn bytes(mut pdf: lopdf::Document) -> Vec<u8> {
        let mut out = Vec::new();
        pdf.save_to(&mut out).unwrap();
        out
    }
    #[test]
    fn inventory_includes_unreachable_nested_and_stream_dictionaries_but_not_stream_bodies() {
        let mut pdf = fixture();
        let action_name = pdf.add_object(Object::Name(b"JavaScript".to_vec()));
        pdf.add_object(dictionary! {"S"=>action_name,"JS"=>Object::string_literal("never executed"),"AA"=>Dictionary::new()});
        pdf.add_object(dictionary! {"S"=>"URI","Type"=>"Sig","Subtype"=>"Screen","XFA"=>Vec::<Object>::new(),"Nested"=>dictionary! {"EF"=>Dictionary::new()}});
        pdf.add_object(Stream::new(
            dictionary! {"Type"=>"EmbeddedFile"},
            b"/S /JavaScript /JS (not a dictionary)".to_vec(),
        ));
        let input = bytes(pdf);
        let original = input.clone();
        let report = inspect(&input, None).unwrap();
        assert!(report.complete, "{:?}", report.limitations);
        assert_eq!(
            report
                .findings
                .iter()
                .map(|f| f.dictionaries)
                .collect::<Vec<_>>(),
            [1, 1, 1, 2, 1, 1, 1]
        );
        assert_eq!(input, original);
        assert_eq!(
            report.source_sha256,
            Sha256::digest(&original)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
        assert_eq!(report.json()["malware_scan"], false);
        assert_eq!(report.json()["signature_integrity_verified"], false);
    }
    #[test]
    fn empty_inventory_is_a_bounded_dictionary_result_not_a_safety_verdict() {
        let report = inspect(&bytes(fixture()), None).unwrap();
        assert!(report.complete);
        assert!(report
            .findings
            .iter()
            .all(|finding| finding.dictionaries == 0));
        assert_eq!(report.json()["conformance_validated"], false);
        assert_eq!(report.json()["stream_contents_examined"], false);
        assert_eq!(report.json()["file_modified"], false);
    }
    #[test]
    fn dangling_and_cyclic_feature_names_are_explicitly_incomplete() {
        let mut pdf = fixture();
        let cycle = pdf.new_object_id();
        pdf.set_object(cycle, Object::Reference(cycle));
        pdf.add_object(dictionary! {"S"=>cycle,"JS"=>Object::Reference((9999,0))});
        let report = inspect(&bytes(pdf), None).unwrap();
        assert!(!report.complete);
        assert!(report
            .limitations
            .contains(&"unresolved_or_malformed_feature_name"));
        assert!(report.limitations.contains(&"unresolved_object_reference"));
        assert_eq!(report.findings[0].dictionaries, 1);
    }
    #[test]
    fn dictionary_depth_and_node_limits_retain_an_incomplete_result() {
        let pdf = fixture();
        let mut report = inspect(&bytes(fixture()), None).unwrap();
        report.visit(&pdf, &Object::Null, MAX_DEPTH + 1);
        assert!(!report.complete);
        report.examined_nodes = MAX_NODES;
        report.visit(&pdf, &Object::Array(vec![Object::Null; 10]), 0);
        assert_eq!(report.examined_nodes, MAX_NODES);
        assert_eq!(report.limitations, ["dictionary_traversal_limit"]);
    }
    #[test]
    fn malformed_input_is_an_error_and_sparse_oversized_files_are_not_loaded() {
        assert!(matches!(
            inspect(b"not a PDF", None),
            Err(EngineError::InvalidFormat)
        ));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("large.pdf");
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(MAX_INPUT_BYTES as u64 + 1).unwrap();
        assert!(inspect_file(&path, None)
            .unwrap_err()
            .to_string()
            .contains("128 MiB"));
        assert_eq!(file.metadata().unwrap().len(), MAX_INPUT_BYTES as u64 + 1);
    }
}
