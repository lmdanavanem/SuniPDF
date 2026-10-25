use lopdf::{dictionary, Document as Pdf, Object, Stream};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex, MutexGuard, OnceLock,
};
use sunipdf_core::{
    annot_rich::VertexAnnotKind, forms::FieldKind, plan_set::PlanSheet, revision_transfer::*,
    security, AnnotShape, AnnotStyle, Engine, EngineError, PointRect, Protection,
};

static SERIAL: Mutex<()> = Mutex::new(());
static ENGINE: OnceLock<EnginePtr> = OnceLock::new();
struct EnginePtr(*const Engine);
// Every access is serialized; the one PDFium instance lives until the test process exits.
unsafe impl Send for EnginePtr {}
unsafe impl Sync for EnginePtr {}
struct SharedEngine {
    _guard: MutexGuard<'static, ()>,
    engine: &'static Engine,
}
impl std::ops::Deref for SharedEngine {
    type Target = Engine;
    fn deref(&self) -> &Engine {
        self.engine
    }
}
fn engine() -> SharedEngine {
    let guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let engine = ENGINE.get_or_init(|| {
        EnginePtr(Box::leak(Box::new(
            Engine::new().expect("PDFium is required"),
        )))
    });
    SharedEngine {
        _guard: guard,
        engine: unsafe { &*engine.0 },
    }
}

fn pdf(rotations: &[i64], origin: f32) -> Vec<u8> {
    let mut pdf = Pdf::with_version("1.7");
    let pages = pdf.new_object_id();
    let mut kids = Vec::new();
    for rotation in rotations {
        let (w, h) = if rotation % 180 == 0 {
            (600.0, 800.0)
        } else {
            (800.0, 600.0)
        };
        let content = pdf.add_object(Stream::new(
            dictionary! {},
            b"q 0.9 g 10 10 5 5 re f Q".to_vec(),
        ));
        let page = pdf.add_object(dictionary! {"Type"=>"Page","Parent"=>pages,
        "MediaBox"=>vec![origin.into(),origin.into(),(origin+w).into(),(origin+h).into()],
        "CropBox"=>vec![origin.into(),origin.into(),(origin+w).into(),(origin+h).into()],
        "Rotate"=>*rotation,"Resources"=>dictionary!{},"Contents"=>content});
        kids.push(Object::Reference(page));
    }
    pdf.objects.insert(
        pages,
        dictionary! {"Type"=>"Pages","Kids"=>kids,"Count"=>rotations.len() as i64}.into(),
    );
    let root = pdf.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
    pdf.trailer.set("Root", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}
fn rect() -> PointRect {
    PointRect {
        left: 40.0,
        bottom: 50.0,
        right: 100.0,
        top: 110.0,
    }
}
fn style() -> AnnotStyle {
    AnnotStyle {
        rgb: (32, 80, 170),
        fill_rgb: None,
        width_pt: 2.0,
        opacity: 0.75,
        dash_pattern: vec![5.0, 3.0],
    }
}
fn fixtures(
    engine: &Engine,
    dir: &Path,
    rotations: &[i64],
    all_shapes: bool,
) -> (PathBuf, PathBuf) {
    let old_path = dir.join("old.pdf");
    let new_path = dir.join("new.pdf");
    let mut old = engine
        .open_bytes(pdf(&vec![0; rotations.len()], 0.0), None, &old_path)
        .unwrap();
    for page in 0..rotations.len() {
        old.add_shape_annotation(
            page,
            AnnotShape::Square,
            rect(),
            &style(),
            Some("Engineer Ω"),
        )
        .unwrap();
        if all_shapes {
            for shape in [AnnotShape::Circle, AnnotShape::Line, AnnotShape::Arrow] {
                old.add_shape_annotation(page, shape, rect(), &style(), Some("Engineer Ω"))
                    .unwrap();
            }
            for kind in [
                VertexAnnotKind::Polygon,
                VertexAnnotKind::Polyline,
                VertexAnnotKind::Cloud,
            ] {
                old.add_vertex_annotation(
                    page,
                    kind,
                    &[(150.0, 200.0), (220.0, 200.0), (190.0, 260.0)],
                    &style(),
                    Some("Engineer Ω"),
                )
                .unwrap();
            }
        }
    }
    old.save_as(&old_path).unwrap();
    let mut new = engine
        .open_bytes(pdf(rotations, 30.0), None, &new_path)
        .unwrap();
    new.add_shape_annotation(
        0,
        AnnotShape::Circle,
        PointRect {
            left: 300.0,
            bottom: 300.0,
            right: 340.0,
            top: 340.0,
        },
        &AnnotStyle::default(),
        Some("Target author"),
    )
    .unwrap();
    new.add_form_field(
        0,
        "KeepField",
        FieldKind::Text,
        PointRect {
            left: 300.0,
            bottom: 400.0,
            right: 400.0,
            top: 430.0,
        },
    )
    .unwrap();
    new.save_as(&new_path).unwrap();
    (old_path, new_path)
}
fn request(inspection: &TransferInspection) -> RevisionTransferRequest {
    RevisionTransferRequest {
        version: 1,
        old_sha256: inspection.old_sha256.clone(),
        new_sha256: inspection.new_sha256.clone(),
        pairs: inspection
            .pages
            .iter()
            .map(|page| TransferPagePair {
                old_page: page.old_page,
                new_page: page.new_page,
                old_sheet_id: None,
                new_sheet_id: None,
                confirmed: true,
                alignment: TransferAlignment::Identity,
                selections: page
                    .candidates
                    .iter()
                    .filter_map(|c| {
                        c.fingerprint.as_ref().map(|hash| TransferSelection {
                            index: c.index,
                            fingerprint: hash.clone(),
                        })
                    })
                    .collect(),
            })
            .collect(),
        acknowledge_repeated_transfer: false,
    }
}
fn inspect(
    engine: &Engine,
    old: &Path,
    new: &Path,
    pairs: &[(usize, usize)],
) -> TransferInspection {
    inspect_transfer_candidates(
        engine,
        old,
        new,
        TransferPasswords::default(),
        pairs,
        &AtomicBool::new(false),
    )
    .unwrap()
}
fn prepare(
    engine: &Engine,
    old: &Path,
    new: &Path,
    request: &RevisionTransferRequest,
) -> PreparedRevisionTransfer {
    prepare_revision_transfer(
        engine,
        old,
        new,
        TransferPasswords::default(),
        request,
        &AtomicBool::new(false),
    )
    .unwrap()
}

#[test]
fn unique_matching_never_infers_revision_order_or_ambiguous_pairs() {
    let sheet = |id: &str, number: &str, revision: &str| PlanSheet {
        id: id.into(),
        file_id: "file-a".into(),
        page: 1,
        number: number.into(),
        title: "Same title".into(),
        discipline: " Architecture ".into(),
        revision: revision.into(),
    };
    let old = vec![sheet("old-a", " A01 ", "Z"), sheet("old-b", "A02", "1")];
    let new = vec![sheet("new-b", "A02", "1"), sheet("new-a", "a01", "A")];
    let pairs = suggest_sheet_pairs(&old, &new).unwrap();
    assert_eq!(pairs[0].new_sheet_id.as_deref(), Some("new-a"));
    assert_eq!(pairs[0].readiness, TransferReadiness::NeedsReview);
    let mut duplicate = new.clone();
    duplicate.push(sheet("new-c", "A01", "later"));
    assert_eq!(
        suggest_sheet_pairs(&old, &duplicate).unwrap()[0].readiness,
        TransferReadiness::Ambiguous
    );
    assert!(suggest_sheet_pairs(&old, &[])
        .unwrap()
        .iter()
        .all(|p| p.readiness == TransferReadiness::Unmatched));
    let mut duplicate_old = old.clone();
    duplicate_old.push(sheet("old-c", "a01", "earlier"));
    assert_eq!(
        suggest_sheet_pairs(&duplicate_old, &new).unwrap()[0].readiness,
        TransferReadiness::Ambiguous
    );
}

#[test]
fn recipes_are_bounded_strict_and_portable() {
    let request = RevisionTransferRequest {
        version: 1,
        old_sha256: "ab".repeat(32),
        new_sha256: "cd".repeat(32),
        pairs: vec![TransferPagePair {
            old_page: 1,
            new_page: 1,
            old_sheet_id: Some("sheet-a".into()),
            new_sheet_id: Some("sheet-b".into()),
            confirmed: false,
            alignment: TransferAlignment::Identity,
            selections: vec![],
        }],
        acknowledge_repeated_transfer: false,
    };
    let encoded = encode_request(&request).unwrap();
    assert_eq!(parse_request(&encoded).unwrap(), request);
    let mut value = serde_json::to_value(&request).unwrap();
    value["password"] = "not accepted".into();
    assert!(parse_request(&serde_json::to_vec(&value).unwrap()).is_err());
    assert!(parse_request(&vec![b' '; MAX_RECIPE_BYTES + 1]).is_err());
    let mut bad = request.clone();
    bad.pairs.push(bad.pairs[0].clone());
    assert!(encode_request(&bad).is_err());
    bad = request.clone();
    bad.pairs[0].old_page = 0;
    assert!(encode_request(&bad).is_err());
    bad = request.clone();
    bad.pairs[0].old_sheet_id = Some("../absolute/path".into());
    assert!(encode_request(&bad).is_err());
    bad = request.clone();
    bad.pairs[0].selections = (0..17)
        .map(|index| TransferSelection {
            index,
            fingerprint: "ef".repeat(32),
        })
        .collect();
    assert!(encode_request(&bad).is_err());
    bad = request.clone();
    bad.pairs = (1..=33)
        .map(|page| {
            let mut pair = request.pairs[0].clone();
            pair.old_page = page;
            pair.new_page = page;
            pair
        })
        .collect();
    assert!(encode_request(&bad).is_err());
    bad = request.clone();
    bad.pairs = (1..=17)
        .map(|page| {
            let mut pair = request.pairs[0].clone();
            pair.old_page = page;
            pair.new_page = page;
            pair.selections = (0..16)
                .map(|index| TransferSelection {
                    index,
                    fingerprint: "ef".repeat(32),
                })
                .collect();
            pair
        })
        .collect();
    assert!(encode_request(&bad).is_err());
    bad = request;
    bad.version = 2;
    assert!(encode_request(&bad).is_err());
}

#[test]
fn native_types_transfer_across_all_page_rotations_preserving_target_and_sources() {
    let engine = engine();
    let dir = tempfile::tempdir().unwrap();
    let (old, new) = fixtures(&engine, dir.path(), &[0, 90, 180, 270], true);
    let old_before = std::fs::read(&old).unwrap();
    let new_before = std::fs::read(&new).unwrap();
    let inspection = inspect(&engine, &old, &new, &[(1, 1), (2, 2), (3, 3), (4, 4)]);
    assert!(inspection.complete());
    for page in &inspection.pages {
        assert_eq!(page.candidates.len(), 7);
        assert!(
            page.candidates.iter().all(|c| c.refusal.is_none()),
            "{:?}",
            page.candidates
        );
    }
    let request = request(&inspection);
    let candidate = prepare(&engine, &old, &new, &request);
    assert_eq!(candidate.report().transferred.len(), 28);
    assert!(candidate.report().output_path.is_none());
    let image = candidate
        .render_preview(&engine, 2, None, 256, &AtomicBool::new(false))
        .unwrap();
    assert!(image.width <= 256 && image.height <= 256);
    assert!(candidate
        .render_preview(&engine, 5, None, 256, &AtomicBool::new(false))
        .is_err());
    assert!(candidate
        .render_preview(&engine, 1, None, 1537, &AtomicBool::new(false))
        .is_err());
    let out = dir.path().join("transferred.pdf");
    let report = publish_revision_transfer(candidate, &out, &AtomicBool::new(false)).unwrap();
    assert!(report.complete());
    assert!(report.output_sha256.is_some());
    let reopened = engine.open(&out, None).unwrap();
    assert_eq!(reopened.page_count(), 4);
    assert!(reopened
        .form_info()
        .unwrap()
        .fields
        .iter()
        .any(|f| f.name == "KeepField"));
    for transfer in &report.transferred {
        assert_ne!(transfer.source_nm, transfer.target_nm);
        assert_eq!(transfer.author.as_deref(), Some("Engineer Ω"));
        let details = reopened.annotation_details(transfer.new_page - 1).unwrap();
        let target = &details[transfer.target_index];
        assert_eq!(target.name.as_deref(), Some(transfer.target_nm.as_str()));
        assert_eq!(target.author.as_deref(), Some("Engineer Ω"));
        assert_eq!(target.dash_pattern, vec![5.0, 3.0]);
        assert_eq!(target.width_pt, Some(2.0));
    }
    assert_eq!(
        reopened.annotation_details(0).unwrap()[0].author.as_deref(),
        Some("Target author")
    );
    assert_eq!(std::fs::read(old).unwrap(), old_before);
    assert_eq!(std::fs::read(new).unwrap(), new_before);
}

#[test]
fn explicit_anchors_use_top_left_direction_scale_width_once_and_reject_unrepresentable_geometry() {
    let engine = engine();
    let dir = tempfile::tempdir().unwrap();
    let (old, new) = fixtures(&engine, dir.path(), &[90], false);
    let mut req = request(&inspect(&engine, &old, &new, &[(1, 1)]));
    req.pairs[0].alignment = TransferAlignment::Anchors {
        old_points: [[40.0, 750.0], [100.0, 750.0]],
        new_points: [[200.0, 500.0], [200.0, 620.0]],
    };
    let candidate = prepare(&engine, &old, &new, &req);
    assert_eq!(candidate.report().pages[0].transform.rotation_degrees, 90.0);
    assert_eq!(candidate.report().pages[0].transform.scale, 2.0);
    let out = dir.path().join("quarter-turn.pdf");
    let report = publish_revision_transfer(candidate, &out, &AtomicBool::new(false)).unwrap();
    let doc = engine.open(&out, None).unwrap();
    let details = doc.annotation_details(0).unwrap();
    let shape = &details[report.transferred[0].target_index];
    assert_eq!(
        shape.rect,
        PointRect {
            left: 200.0,
            bottom: 180.0,
            right: 320.0,
            top: 300.0
        }
    );
    assert_eq!(shape.width_pt, Some(4.0));
    assert_eq!(shape.dash_pattern, vec![10.0, 6.0]);
    let mut bad = req.clone();
    bad.pairs[0].alignment = TransferAlignment::Anchors {
        old_points: [[40.0, 750.0], [100.0, 750.0]],
        new_points: [[200.0, 500.0], [260.0, 560.0]],
    };
    assert!(prepare_revision_transfer(
        &engine,
        &old,
        &new,
        TransferPasswords::default(),
        &bad,
        &AtomicBool::new(false)
    )
    .unwrap_err()
    .to_string()
    .contains("quarter-turn"));
    bad.pairs[0].alignment = TransferAlignment::Anchors {
        old_points: [[40.0, 750.0], [100.0, 750.0]],
        new_points: [[590.0, 500.0], [590.0, 620.0]],
    };
    assert!(prepare_revision_transfer(
        &engine,
        &old,
        &new,
        TransferPasswords::default(),
        &bad,
        &AtomicBool::new(false)
    )
    .is_err());
    bad.pairs[0].alignment = TransferAlignment::Anchors {
        old_points: [[-1.0, 1.0], [100.0, 1.0]],
        new_points: [[0.0, 1.0], [100.0, 1.0]],
    };
    assert!(prepare_revision_transfer(
        &engine,
        &old,
        &new,
        TransferPasswords::default(),
        &bad,
        &AtomicBool::new(false)
    )
    .is_err());
    // Large crop offsets can move coordinates despite equal sizes; no output may be committed.
    std::fs::write(&new, pdf(&[0], 16_777_216.0)).unwrap();
    let mut old_doc = engine.open(&old, None).unwrap();
    old_doc
        .add_shape_annotation(
            0,
            AnnotShape::Square,
            PointRect {
                left: 60.75,
                bottom: 100.75,
                right: 80.75,
                top: 150.75,
            },
            &style(),
            None,
        )
        .unwrap();
    old_doc.save_as(&old).unwrap();
    drop(old_doc);
    let bad = request(&inspect(&engine, &old, &new, &[(1, 1)]));
    assert!(prepare_revision_transfer(
        &engine,
        &old,
        &new,
        TransferPasswords::default(),
        &bad,
        &AtomicBool::new(false)
    )
    .unwrap_err()
    .to_string()
    .contains("0.001"));
}

#[test]
fn source_changes_existing_outputs_repetition_and_cancellation_never_overwrite() {
    let engine = engine();
    let dir = tempfile::tempdir().unwrap();
    let (old, new) = fixtures(&engine, dir.path(), &[0], false);
    let same = request(&inspect(&engine, &old, &old, &[(1, 1)]));
    let absent = dir.path().join("identical-must-not-exist.pdf");
    let attempt = prepare_revision_transfer(
        &engine,
        &old,
        &old,
        TransferPasswords::default(),
        &same,
        &AtomicBool::new(false),
    )
    .and_then(|candidate| publish_revision_transfer(candidate, &absent, &AtomicBool::new(false)));
    assert!(attempt
        .unwrap_err()
        .to_string()
        .contains("identical saved revisions"));
    assert!(!absent.exists());
    let identical_copy = dir.path().join("same-bytes.pdf");
    std::fs::copy(&old, &identical_copy).unwrap();
    assert!(prepare_revision_transfer(
        &engine,
        &old,
        &identical_copy,
        TransferPasswords::default(),
        &same,
        &AtomicBool::new(false)
    )
    .unwrap_err()
    .to_string()
    .contains("identical saved revisions"));
    let mut req = request(&inspect(&engine, &old, &new, &[(1, 1)]));
    req.pairs[0].confirmed = false;
    assert!(prepare_revision_transfer(
        &engine,
        &old,
        &new,
        TransferPasswords::default(),
        &req,
        &AtomicBool::new(false)
    )
    .is_err());
    req.pairs[0].confirmed = true;
    let mut forged = req.clone();
    forged.pairs[0].selections[0].fingerprint = "ef".repeat(32);
    assert!(prepare_revision_transfer(
        &engine,
        &old,
        &new,
        TransferPasswords::default(),
        &forged,
        &AtomicBool::new(false)
    )
    .is_err());
    let original = std::fs::read(&old).unwrap();
    let candidate = prepare(&engine, &old, &new, &req);
    let mut changed = original.clone();
    changed.extend_from_slice(b"\n% revision changed\n");
    std::fs::write(&old, &changed).unwrap();
    let out = dir.path().join("out.pdf");
    assert!(publish_revision_transfer(candidate, &out, &AtomicBool::new(false)).is_err());
    assert!(!out.exists());
    std::fs::write(&old, &original).unwrap();
    let candidate = prepare(&engine, &old, &new, &req);
    std::fs::write(&out, b"existing output").unwrap();
    assert!(matches!(
        publish_revision_transfer(candidate, &out, &AtomicBool::new(false)),
        Err(EngineError::AlreadyExists(_))
    ));
    assert_eq!(std::fs::read(&out).unwrap(), b"existing output");
    let alias = dir.path().join("source-alias.pdf");
    std::fs::hard_link(&old, &alias).unwrap();
    assert!(matches!(
        publish_revision_transfer(
            prepare(&engine, &old, &new, &req),
            &alias,
            &AtomicBool::new(false)
        ),
        Err(EngineError::AlreadyExists(_))
    ));
    assert!(matches!(
        publish_revision_transfer(
            prepare(&engine, &old, &new, &req),
            &old,
            &AtomicBool::new(false)
        ),
        Err(EngineError::AlreadyExists(_))
    ));
    let cancelled = AtomicBool::new(true);
    assert!(matches!(
        prepare_revision_transfer(
            &engine,
            &old,
            &new,
            TransferPasswords::default(),
            &req,
            &cancelled
        ),
        Err(EngineError::Cancelled)
    ));
    let cancelled_out = dir.path().join("cancelled.pdf");
    assert!(matches!(
        publish_revision_transfer(
            prepare(&engine, &old, &new, &req),
            &cancelled_out,
            &cancelled
        ),
        Err(EngineError::Cancelled)
    ));
    assert!(!cancelled_out.exists());
    let first = dir.path().join("first.pdf");
    publish_revision_transfer(
        prepare(&engine, &old, &new, &req),
        &first,
        &AtomicBool::new(false),
    )
    .unwrap();
    let inspection = inspect(&engine, &old, &first, &[(1, 1)]);
    assert!(!inspection.warnings.is_empty());
    let mut repeated = request(&inspection);
    assert!(prepare_revision_transfer(
        &engine,
        &old,
        &first,
        TransferPasswords::default(),
        &repeated,
        &AtomicBool::new(false)
    )
    .is_err());
    repeated.acknowledge_repeated_transfer = true;
    let candidate = prepare(&engine, &old, &first, &repeated);
    let second = dir.path().join("second.pdf");
    let report = publish_revision_transfer(candidate, &second, &AtomicBool::new(false)).unwrap();
    cancelled.store(false, Ordering::Relaxed);
    assert!(report.complete());
    assert!(second.exists());
    assert_eq!(std::fs::read(old).unwrap(), original);
}

#[test]
fn unsupported_custom_fields_are_excluded_without_hiding_compatible_shapes() {
    let engine = engine();
    let dir = tempfile::tempdir().unwrap();
    let (old, new) = fixtures(&engine, dir.path(), &[0], true);
    let mut pdf = Pdf::load(&old).unwrap();
    let page = *pdf.get_pages().get(&1).unwrap();
    let first = pdf
        .get_dictionary(page)
        .unwrap()
        .get(b"Annots")
        .unwrap()
        .as_array()
        .unwrap()[0]
        .as_reference()
        .unwrap();
    pdf.get_dictionary_mut(first).unwrap().set(
        "SuniPDFMeasurement",
        Object::string_literal("unsupported measurement semantics"),
    );
    pdf.save(&old).unwrap();
    let inspection = inspect(&engine, &old, &new, &[(1, 1)]);
    assert!(inspection.complete());
    assert!(inspection.pages[0].candidates[0].refusal.is_some());
    let request = request(&inspection);
    let candidate = prepare(&engine, &old, &new, &request);
    assert_eq!(candidate.report().transferred.len(), 6);
    assert_eq!(candidate.report().exclusions.len(), 1);
    assert_eq!(candidate.report().exclusions[0].index, 0);
    assert!(!candidate.report().exclusions[0].reason.is_empty());
}

#[test]
fn encrypted_inputs_keep_target_protection_and_enforce_permissions() {
    let engine = engine();
    let dir = tempfile::tempdir().unwrap();
    let (old, new) = fixtures(&engine, dir.path(), &[0], false);
    let old_bytes = std::fs::read(&old).unwrap();
    let new_bytes = std::fs::read(&new).unwrap();
    for aes256 in [false, true] {
        let options = Protection {
            user_password: "reader".into(),
            owner_password: "owner".into(),
            allow_print: false,
            allow_copy: true,
            allow_modify: false,
            allow_annotate: true,
            aes256,
        };
        std::fs::write(&old, security::protect(&old_bytes, None, &options).unwrap()).unwrap();
        std::fs::write(&new, security::protect(&new_bytes, None, &options).unwrap()).unwrap();
        let passwords = TransferPasswords {
            old: Some("reader"),
            new: Some("reader"),
        };
        let cancel = AtomicBool::new(false);
        let inspection =
            inspect_transfer_candidates(&engine, &old, &new, passwords, &[(1, 1)], &cancel)
                .unwrap();
        let req = request(&inspection);
        let candidate =
            prepare_revision_transfer(&engine, &old, &new, passwords, &req, &cancel).unwrap();
        assert!(candidate
            .render_preview(&engine, 1, None, 128, &cancel)
            .is_err());
        assert!(candidate
            .render_preview(&engine, 1, Some("reader"), 128, &cancel)
            .is_ok());
        let out = dir.path().join(format!("encrypted-{aes256}.pdf"));
        publish_revision_transfer(candidate, &out, &cancel).unwrap();
        assert!(engine.open(&out, None).is_err());
        let doc = engine.open(&out, Some("reader")).unwrap();
        assert!(!doc.properties().can_print);
        assert!(!doc.properties().can_modify);
        assert!(doc.properties().can_annotate);
        drop(doc);
        let mut no_copy = options.clone();
        no_copy.allow_copy = false;
        std::fs::write(&old, security::protect(&old_bytes, None, &no_copy).unwrap()).unwrap();
        assert!(matches!(
            inspect_transfer_candidates(&engine, &old, &new, passwords, &[(1, 1)], &cancel),
            Err(EngineError::SecurityRestricted)
        ));
        std::fs::write(&old, security::protect(&old_bytes, None, &options).unwrap()).unwrap();
        let mut no_annotate = options;
        no_annotate.allow_annotate = false;
        std::fs::write(
            &new,
            security::protect(&new_bytes, None, &no_annotate).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            inspect_transfer_candidates(&engine, &old, &new, passwords, &[(1, 1)], &cancel),
            Err(EngineError::SecurityRestricted)
        ));
    }
}

#[test]
fn unsupported_semantics_and_parent_threads_are_reported_signed_targets_are_refused() {
    let engine = engine();
    let dir = tempfile::tempdir().unwrap();
    let (old, new) = fixtures(&engine, dir.path(), &[0], false);
    let mut doc = engine.open(&old, None).unwrap();
    doc.add_reply(0, 0, "Review history", Some("Reviewer"))
        .unwrap();
    doc.save_as(&old).unwrap();
    drop(doc);
    let inspection = inspect(&engine, &old, &new, &[(1, 1)]);
    assert!(!inspection.complete());
    assert!(inspection.pages[0].candidates[0]
        .refusal
        .as_ref()
        .unwrap()
        .contains("history"));
    assert!(inspection.pages[0]
        .candidates
        .iter()
        .all(|c| c.refusal.is_some()));
    let mut pdf = Pdf::load(&new).unwrap();
    pdf.add_object(dictionary!{"Type"=>"Sig","ByteRange"=>vec![0.into(),0.into(),0.into(),0.into()],"Contents"=>Object::string_literal("invalid signature")});
    pdf.save(&new).unwrap();
    assert!(inspect_transfer_candidates(
        &engine,
        &old,
        &new,
        TransferPasswords::default(),
        &[(1, 1)],
        &AtomicBool::new(false)
    )
    .unwrap_err()
    .to_string()
    .contains("signed target"));
}

#[test]
fn cross_page_reply_threads_are_refused_by_library_capture_and_revision_transfer() {
    use sunipdf_core::annotation_library::{LibraryCaptureSelection, LibraryProvenance};
    let engine = engine();
    let dir = tempfile::tempdir().unwrap();
    let (old, new) = fixtures(&engine, dir.path(), &[0, 0], false);
    let mut doc = engine.open(&old, None).unwrap();
    let fingerprint = doc.library_capture_candidates(0).unwrap()[0]
        .fingerprint
        .clone()
        .unwrap();
    doc.add_reply(0, 0, "A cross-page review reply", Some("Reviewer"))
        .unwrap();
    let with_reply = doc.snapshot().unwrap();
    drop(doc);
    for direct in [false, true] {
        let mut pdf = Pdf::load_mem(&with_reply).unwrap();
        let pages = pdf.get_pages();
        let first = pages[&1];
        let second = pages[&2];
        let mut first_annots = pdf
            .get_dictionary(first)
            .unwrap()
            .get(b"Annots")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        let reply = first_annots.pop().unwrap();
        let id = reply.as_reference().unwrap();
        pdf.get_dictionary_mut(first)
            .unwrap()
            .set("Annots", first_annots);
        pdf.get_dictionary_mut(id).unwrap().set("P", second);
        let reply = if direct {
            pdf.objects.remove(&id).unwrap()
        } else {
            reply
        };
        let mut second_annots = pdf
            .get_dictionary(second)
            .unwrap()
            .get(b"Annots")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        second_annots.push(reply);
        pdf.get_dictionary_mut(second)
            .unwrap()
            .set("Annots", second_annots);
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        std::fs::write(&old, &bytes).unwrap();
        let doc = engine.open(&old, None).unwrap();
        assert!(doc.library_capture_candidates(0).unwrap()[0]
            .refusal
            .as_deref()
            .is_some_and(|s| s.contains("history")));
        assert!(doc
            .capture_library_entry(
                0,
                &[LibraryCaptureSelection {
                    index: 0,
                    fingerprint: fingerprint.clone()
                }],
                "review".into(),
                "Review".into(),
                LibraryProvenance {
                    source: Some("Synthetic fixture".into()),
                    ..Default::default()
                }
            )
            .is_err());
        drop(doc);
        let inspection = inspect(&engine, &old, &new, &[(1, 1)]);
        assert!(!inspection.complete());
        let mut req = request(&inspection);
        req.pairs[0].selections.push(TransferSelection {
            index: 0,
            fingerprint: fingerprint.clone(),
        });
        let out = dir.path().join(format!("refused-thread-{direct}.pdf"));
        let result = prepare_revision_transfer(
            &engine,
            &old,
            &new,
            TransferPasswords::default(),
            &req,
            &AtomicBool::new(false),
        )
        .and_then(|candidate| publish_revision_transfer(candidate, &out, &AtomicBool::new(false)));
        assert!(result.unwrap_err().to_string().contains("history"));
        assert!(!out.exists());
        assert_eq!(std::fs::read(&old).unwrap(), bytes);
    }
}

#[test]
fn sixty_fourth_transfer_remains_inspectable_and_sixty_fifth_is_refused() {
    let engine = engine();
    let dir = tempfile::tempdir().unwrap();
    let (old, new) = fixtures(&engine, dir.path(), &[0], false);
    let mut pdf = Pdf::load(&new).unwrap();
    let history: Vec<_> = (0..63)
        .map(|_| serde_json::json!({"old_sha256":"0".repeat(64),"annotations":1}))
        .collect();
    pdf.catalog_mut().unwrap().set(
        "SuniPDFRevisionTransfers",
        Object::string_literal(serde_json::to_vec(&history).unwrap()),
    );
    pdf.save(&new).unwrap();
    let req = request(&inspect(&engine, &old, &new, &[(1, 1)]));
    let out = dir.path().join("history-64.pdf");
    publish_revision_transfer(
        prepare(&engine, &old, &new, &req),
        &out,
        &AtomicBool::new(false),
    )
    .unwrap();
    let inspection = inspect(&engine, &old, &out, &[(1, 1)]);
    assert!(inspection.complete());
    let mut pdf = Pdf::load(&out).unwrap();
    let raw = pdf
        .catalog()
        .unwrap()
        .get(b"SuniPDFRevisionTransfers")
        .unwrap()
        .as_str()
        .unwrap();
    let mut history: Vec<serde_json::Value> = serde_json::from_slice(raw).unwrap();
    assert_eq!(history.len(), 64);
    let mut req = request(&inspection);
    req.acknowledge_repeated_transfer = true;
    let refused = dir.path().join("history-65.pdf");
    let result = prepare_revision_transfer(
        &engine,
        &old,
        &out,
        TransferPasswords::default(),
        &req,
        &AtomicBool::new(false),
    )
    .and_then(|candidate| publish_revision_transfer(candidate, &refused, &AtomicBool::new(false)));
    assert!(result.unwrap_err().to_string().contains("maximum of 64"));
    assert!(!refused.exists());
    history.push(history[0].clone());
    pdf.catalog_mut().unwrap().set(
        "SuniPDFRevisionTransfers",
        Object::string_literal(serde_json::to_vec(&history).unwrap()),
    );
    let invalid = dir.path().join("invalid-history.pdf");
    pdf.save(&invalid).unwrap();
    assert!(inspect_transfer_candidates(
        &engine,
        &old,
        &invalid,
        TransferPasswords::default(),
        &[(1, 1)],
        &AtomicBool::new(false)
    )
    .is_err());
}

#[cfg(any(unix, windows))]
fn directory_alias(link: &Path, target: &Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).unwrap();
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Unprivileged directory junction; both paths belong to this test's temporary directory.
        let output = std::process::Command::new("cmd.exe")
            .creation_flags(0x0800_0000)
            .args(["/D", "/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "junction fixture: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[cfg(any(unix, windows))]
fn remove_directory_alias(link: &Path) {
    #[cfg(unix)]
    std::fs::remove_file(link).unwrap();
    #[cfg(windows)]
    std::fs::remove_dir(link).unwrap();
}

#[cfg(any(unix, windows))]
#[test]
fn selected_source_alias_retarget_invalidates_preview_and_publication_on_both_sides() {
    let engine = engine();
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first");
    let second = dir.path().join("second");
    std::fs::create_dir(&first).unwrap();
    std::fs::create_dir(&second).unwrap();
    let (old, new) = fixtures(&engine, &first, &[0], false);
    // Equal bytes under a different canonical target still represent a changed chosen path.
    std::fs::copy(&old, second.join("old.pdf")).unwrap();
    std::fs::copy(&new, second.join("new.pdf")).unwrap();
    let old_before = std::fs::read(&old).unwrap();
    let new_before = std::fs::read(&new).unwrap();
    for old_alias in [false, true] {
        let link = dir
            .path()
            .join(if old_alias { "old-alias" } else { "new-alias" });
        directory_alias(&link, &first);
        let selected_old = if old_alias {
            link.join("old.pdf")
        } else {
            old.clone()
        };
        let selected_new = if old_alias {
            new.clone()
        } else {
            link.join("new.pdf")
        };
        let req = request(&inspect(&engine, &selected_old, &selected_new, &[(1, 1)]));
        let candidate = prepare(&engine, &selected_old, &selected_new, &req);
        candidate
            .revalidate_sources(&AtomicBool::new(false))
            .unwrap();
        remove_directory_alias(&link);
        directory_alias(&link, &second);
        assert!(candidate
            .revalidate_sources(&AtomicBool::new(false))
            .unwrap_err()
            .to_string()
            .contains("selected source path changed"));
        assert!(candidate
            .render_preview(&engine, 1, None, 128, &AtomicBool::new(false))
            .is_err());
        let out = dir.path().join(format!("alias-refused-{old_alias}.pdf"));
        assert!(publish_revision_transfer(candidate, &out, &AtomicBool::new(false)).is_err());
        assert!(!out.exists());
        remove_directory_alias(&link);
    }
    assert_eq!(std::fs::read(old).unwrap(), old_before);
    assert_eq!(std::fs::read(new).unwrap(), new_before);
}

#[test]
fn reviewed_batch_reopens_two_real_revisions_with_existing_fields_and_new_annotation_ids() {
    use sunipdf_core::revision_transfer::batch::PreparedTransferBatch;
    let engine = engine();
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let cancel = AtomicBool::new(false);
    let mut batch = PreparedTransferBatch::new();
    let mut sources = Vec::new();
    for (id, rotations) in [("first", vec![0, 90]), ("second", vec![180])] {
        let directory = root.join(id);
        std::fs::create_dir(&directory).unwrap();
        let (old, new) = fixtures(&engine, &directory, &rotations, true);
        sources.push((old.clone(), std::fs::read(&old).unwrap()));
        sources.push((new.clone(), std::fs::read(&new).unwrap()));
        let pairs: Vec<_> = (1..=rotations.len()).map(|page| (page, page)).collect();
        let request = request(&inspect(&engine, &old, &new, &pairs));
        let prepared = prepare(&engine, &old, &new, &request);
        batch
            .push(id.into(), prepared, root.join(format!("{id}-reviewed.pdf")))
            .unwrap();
    }
    assert_eq!(batch.len(), 2);
    batch.preflight(&cancel).unwrap();
    assert!(!root.join("first-reviewed.pdf").exists());
    assert!(!root.join("second-reviewed.pdf").exists());
    let preview = batch
        .render_preview("first", &engine, 2, None, 256, &cancel)
        .unwrap();
    assert!(preview.width > 0 && preview.height > 0);
    assert!(preview.width <= 256 && preview.height <= 256);
    assert!(batch
        .render_preview("missing", &engine, 1, None, 256, &cancel)
        .is_err());
    let outcome = batch.publish(&cancel, |_| {});
    assert!(outcome.complete());
    assert_eq!(outcome.completed.len(), 2);
    assert!(outcome.remaining.is_empty());
    for (completed, expected_pages) in outcome.completed.iter().zip([2, 1]) {
        let report = &completed.report;
        assert!(report.complete());
        assert_eq!(report.pages.len(), expected_pages);
        assert_eq!(report.transferred.len(), expected_pages * 7);
        let output = report.output_path.as_ref().unwrap();
        let reopened = engine.open(output, None).unwrap();
        assert_eq!(reopened.page_count(), expected_pages);
        assert!(reopened
            .form_info()
            .unwrap()
            .fields
            .iter()
            .any(|f| f.name == "KeepField"));
        assert_eq!(
            reopened.annotation_details(0).unwrap()[0].author.as_deref(),
            Some("Target author")
        );
        for transfer in &report.transferred {
            assert_ne!(transfer.source_nm, transfer.target_nm);
            let annotations = reopened.annotation_details(transfer.new_page - 1).unwrap();
            let written = &annotations[transfer.target_index];
            assert_eq!(written.name.as_deref(), Some(transfer.target_nm.as_str()));
            assert_eq!(written.author.as_deref(), Some("Engineer Ω"));
            assert_eq!(written.dash_pattern, vec![5.0, 3.0]);
        }
    }
    for (path, original) in sources {
        assert_eq!(std::fs::read(path).unwrap(), original);
    }
}

#[test]
fn reviewed_batch_refuses_a_changed_later_pdf_before_creating_any_copy() {
    use sunipdf_core::revision_transfer::batch::{PreparedTransferBatch, TransferBatchStatus};
    let engine = engine();
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let mut batch = PreparedTransferBatch::new();
    for id in ["first", "second"] {
        let directory = root.join(id);
        std::fs::create_dir(&directory).unwrap();
        let (old, new) = fixtures(&engine, &directory, &[0], false);
        let request = request(&inspect(&engine, &old, &new, &[(1, 1)]));
        batch
            .push(
                id.into(),
                prepare(&engine, &old, &new, &request),
                root.join(format!("{id}-reviewed.pdf")),
            )
            .unwrap();
    }
    let later = root.join("second/new.pdf");
    let mut changed = engine.open(&later, None).unwrap();
    changed
        .add_shape_annotation(0, AnnotShape::Square, rect(), &style(), Some("Later edit"))
        .unwrap();
    changed.save_as(&later).unwrap();
    drop(changed);
    let outcome = batch.publish(&AtomicBool::new(false), |_| {
        panic!("all sources must be preflighted")
    });
    assert_eq!(outcome.status, TransferBatchStatus::Failed);
    assert_eq!(
        outcome.failure.as_ref().unwrap().id.as_deref(),
        Some("second")
    );
    assert!(outcome.completed.is_empty());
    assert_eq!(outcome.remaining.len(), 2);
    assert!(!root.join("first-reviewed.pdf").exists());
    assert!(!root.join("second-reviewed.pdf").exists());
}

#[test]
fn reviewed_batch_keeps_each_encryption_mode_without_retaining_credentials() {
    use sunipdf_core::revision_transfer::batch::PreparedTransferBatch;
    let engine = engine();
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let cancel = AtomicBool::new(false);
    let mut batch = PreparedTransferBatch::new();
    let reader_password = ["batch", "reader", "fixture"].join("-");
    let owner_password = ["batch", "owner", "fixture"].join("-");
    for (id, aes256) in [("aes128", false), ("aes256", true)] {
        let directory = root.join(id);
        std::fs::create_dir(&directory).unwrap();
        let (old, new) = fixtures(&engine, &directory, &[0], false);
        let protection = Protection {
            user_password: reader_password.clone(),
            owner_password: owner_password.clone(),
            allow_print: false,
            allow_copy: true,
            allow_modify: false,
            allow_annotate: true,
            aes256,
        };
        for path in [&old, &new] {
            let cleartext = std::fs::read(path).unwrap();
            std::fs::write(
                path,
                security::protect(&cleartext, None, &protection).unwrap(),
            )
            .unwrap();
        }
        let credentials = TransferPasswords {
            old: Some(&reader_password),
            new: Some(&reader_password),
        };
        let inspection =
            inspect_transfer_candidates(&engine, &old, &new, credentials, &[(1, 1)], &cancel)
                .unwrap();
        let prepared = prepare_revision_transfer(
            &engine,
            &old,
            &new,
            credentials,
            &request(&inspection),
            &cancel,
        )
        .unwrap();
        batch
            .push(id.into(), prepared, root.join(format!("{id}-reviewed.pdf")))
            .unwrap();
    }
    let summary_json = serde_json::to_string(&batch.summaries()).unwrap();
    assert!(!summary_json.contains(&reader_password));
    assert!(!summary_json.contains(&owner_password));
    assert!(batch
        .render_preview("aes256", &engine, 1, None, 128, &cancel)
        .is_err());
    assert!(batch
        .render_preview("aes256", &engine, 1, Some(&reader_password), 128, &cancel)
        .is_ok());
    let outcome = batch.publish(&cancel, |_| {});
    assert!(outcome.complete());
    assert_eq!(outcome.completed.len(), 2);
    let outcome_json = outcome.json().to_string();
    assert!(!outcome_json.contains(&reader_password));
    assert!(!outcome_json.contains(&owner_password));
    for completed in outcome.completed {
        let output = completed.report.output_path.unwrap();
        assert!(engine.open(&output, None).is_err());
        let reopened = engine.open(&output, Some(&reader_password)).unwrap();
        assert!(!reopened.properties().can_print);
        assert!(!reopened.properties().can_modify);
        assert!(reopened.properties().can_annotate);
        assert_eq!(completed.report.transferred.len(), 1);
    }
}
