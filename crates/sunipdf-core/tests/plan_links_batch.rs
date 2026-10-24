use lopdf::{dictionary, Document as Pdf, Object, Stream};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex, MutexGuard, OnceLock,
    },
};
use sunipdf_core::{plan_links::*, plan_set::*, security, Engine, LinkTarget, Protection};

static SERIAL: Mutex<()> = Mutex::new(());
static ENGINE: OnceLock<EnginePtr> = OnceLock::new();
struct EnginePtr(*const Engine);
// PDFium access is serialized for the lifetime of this test process.
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
    let pointer = ENGINE
        .get_or_init(|| EnginePtr(Box::leak(Box::new(Engine::new().expect("PDFium required")))));
    SharedEngine {
        _guard: guard,
        engine: unsafe { &*pointer.0 },
    }
}
fn pdf(text: &str) -> Vec<u8> {
    let mut pdf = Pdf::with_version("1.7");
    let pages = pdf.new_object_id();
    let font =
        pdf.add_object(dictionary! {"Type"=>"Font", "Subtype"=>"Type1", "BaseFont"=>"Helvetica"});
    let content = pdf.add_object(Stream::new(
        dictionary! {},
        format!("BT /F1 12 Tf 50 200 Td ({text}) Tj ET").into_bytes(),
    ));
    let page = pdf.add_object(dictionary! {"Type"=>"Page", "Parent"=>pages,
    "MediaBox"=>vec![0.into(),0.into(),600.into(),800.into()], "Contents"=>content,
    "Resources"=>dictionary! {"Font"=>dictionary! {"F1"=>font}}});
    pdf.objects.insert(
        pages,
        dictionary! {"Type"=>"Pages", "Count"=>1, "Kids"=>vec![Object::Reference(page)]}.into(),
    );
    let root = pdf.add_object(dictionary! {"Type"=>"Catalog", "Pages"=>pages});
    pdf.trailer.set("Root", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}
struct Fixture {
    dir: tempfile::TempDir,
    manifest: PathBuf,
    set: PlanSet,
    request: PlanLinkBatchScanRequest,
}
fn fixture(engine: &Engine) -> Fixture {
    fixture_source(engine, None, None)
}
fn fixture_source(engine: &Engine, source: Option<&[u8]>, password: Option<&str>) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let manifest = dir.path().join("plans.json");
    let mut set = PlanSet::new("Linked plans").unwrap();
    let mut sheets = Vec::new();
    for (filename, text, number) in [("A.pdf", "B-20", "A-10"), ("B.pdf", "A-10", "B-20")] {
        let path = dir.path().join(filename);
        std::fs::write(
            &path,
            if filename == "A.pdf" {
                source.map(Vec::from).unwrap_or_else(|| pdf(text))
            } else {
                pdf(text)
            },
        )
        .unwrap();
        let ids = add_pdf(
            &mut set,
            engine,
            &manifest,
            &path,
            if filename == "A.pdf" { password } else { None },
            &AddOptions::default(),
        )
        .unwrap();
        set.sheets
            .iter_mut()
            .find(|s| s.id == ids[0])
            .unwrap()
            .number = number.into();
        sheets.push(ids[0].clone());
    }
    let request = PlanLinkBatchScanRequest {
        sources: (0..2)
            .map(|i| PlanLinkScanRequest {
                source_file_id: set
                    .sheets
                    .iter()
                    .find(|s| s.id == sheets[i])
                    .unwrap()
                    .file_id
                    .clone(),
                pages: vec![1],
                target_sheet_ids: vec![sheets[1 - i].clone()],
            })
            .collect(),
    };
    std::fs::write(&manifest, encode(&set).unwrap()).unwrap();
    Fixture {
        dir,
        manifest,
        set,
        request,
    }
}
fn review(engine: &Engine, f: &Fixture) -> PlanLinkBatchReview {
    let inspection = inspect_plan_link_batch(
        engine,
        &f.manifest,
        &f.set,
        &f.request,
        &PlanLinkCredentials::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(inspection.complete(), "{inspection:?}");
    assert_eq!(inspection.occurrences, 2);
    // Equal occurrence IDs in separate PDFs remain scoped by each source review.
    assert_eq!(
        inspection.files[0].occurrences[0].id,
        inspection.files[1].occurrences[0].id
    );
    reviewed(f, inspection)
}
fn reviewed(f: &Fixture, inspection: PlanLinkBatchInspection) -> PlanLinkBatchReview {
    PlanLinkBatchReview {
        version: 1,
        manifest_sha256: inspection.manifest_sha256,
        files: inspection
            .files
            .into_iter()
            .enumerate()
            .map(|(i, report)| PlanLinkBatchFileReview {
                output_name: format!("linked-{i}.pdf"),
                review: PlanLinkReview {
                    version: 1,
                    manifest_sha256: report.manifest_sha256,
                    source_sha256: report.source_sha256,
                    scan: f.request.sources[i].clone(),
                    selections: report
                        .occurrences
                        .into_iter()
                        .map(|o| PlanLinkSelection {
                            occurrence_id: o.id,
                            fingerprint: o.fingerprint,
                            target_sheet_id: o.target_sheet_ids[0].clone(),
                            confirmed: true,
                        })
                        .collect(),
                },
            })
            .collect(),
    }
}
fn prepare(engine: &Engine, f: &Fixture, review: &PlanLinkBatchReview) -> PlanLinkBatchPlan {
    prepare_plan_link_batch(
        engine,
        &f.manifest,
        &f.set,
        review,
        &PlanLinkCredentials::default(),
        &AtomicBool::new(false),
    )
    .unwrap()
}

#[test]
fn mutual_links_use_originals_and_keep_original_bytes_and_scoped_occurrences() {
    let engine = engine();
    let f = fixture(&engine);
    let a = std::fs::read(f.dir.path().join("A.pdf")).unwrap();
    let b = std::fs::read(f.dir.path().join("B.pdf")).unwrap();
    let review = review(&engine, &f);
    let encoded = encode_batch_review(&review).unwrap();
    let review = parse_batch_review(&encoded).unwrap();
    let plan = prepare(&engine, &f, &review);
    assert_eq!(plan.files().len(), 2);
    assert_eq!(plan.files()[0].preview_pages, [1]);
    let source_image = plan
        .preview_file(
            0,
            &engine,
            &PlanLinkCredentials::default(),
            1,
            128,
            &AtomicBool::new(false),
        )
        .unwrap();
    assert!(!source_image.rgba.is_empty());
    let target_image = plan
        .preview_target(
            0,
            &engine,
            &PlanLinkCredentials::default(),
            &review.files[0].review.selections[0].target_sheet_id,
            128,
            &AtomicBool::new(false),
        )
        .unwrap();
    assert!(!target_image.rgba.is_empty());
    let mut progress = Vec::new();
    let result = publish_plan_link_batch(
        plan,
        &engine,
        &PlanLinkCredentials::default(),
        &AtomicBool::new(false),
        |p| progress.push(p.clone()),
    );
    assert!(result.complete(), "{result:?}");
    assert_eq!(progress.len(), 2);
    assert!(result.remaining.is_empty());
    assert!(result.last_input_validation.as_ref().unwrap().valid);
    for (i, receipt) in result.completed.iter().enumerate() {
        assert_eq!(receipt.report.created.len(), 1);
        let target = &receipt.report.created[0].target;
        assert_eq!(target.relative_path, if i == 0 { "B.pdf" } else { "A.pdf" });
        let doc = engine
            .open_bytes(
                std::fs::read(&receipt.output).unwrap(),
                None,
                &receipt.output,
            )
            .unwrap();
        assert!(doc
            .page_links(0)
            .unwrap()
            .iter()
            .any(|link| link.target == LinkTarget::RemotePage(target.clone())));
        assert_eq!(receipt.report.output_sha256.len(), 64);
    }
    assert_eq!(a, std::fs::read(f.dir.path().join("A.pdf")).unwrap());
    assert_eq!(b, std::fs::read(f.dir.path().join("B.pdf")).unwrap());
    assert_eq!(parse(&std::fs::read(&f.manifest).unwrap()).unwrap(), f.set);
}

#[test]
fn all_output_names_and_review_confirmations_are_preflighted_before_any_save() {
    let engine = engine();
    let f = fixture(&engine);
    let original = review(&engine, &f);
    for name in [
        "../escape.pdf",
        "nested/new.pdf",
        "A.pdf",
        "a.PDF",
        "linked-0.PDF",
        "CON.pdf",
    ] {
        let mut review = original.clone();
        review.files[1].output_name = name.into();
        assert!(
            prepare_plan_link_batch(
                &engine,
                &f.manifest,
                &f.set,
                &review,
                &PlanLinkCredentials::default(),
                &AtomicBool::new(false)
            )
            .is_err(),
            "{name}"
        );
        assert!(!f.dir.path().join("linked-0.pdf").exists());
    }
    let mut unconfirmed = original.clone();
    unconfirmed.files[1].review.selections[0].confirmed = false;
    assert!(prepare_plan_link_batch(
        &engine,
        &f.manifest,
        &f.set,
        &unconfirmed,
        &PlanLinkCredentials::default(),
        &AtomicBool::new(false)
    )
    .is_err());
    let mut wrong_scope = original.clone();
    wrong_scope.files[1].review.selections[0].fingerprint =
        original.files[0].review.selections[0].fingerprint.clone();
    assert!(prepare_plan_link_batch(
        &engine,
        &f.manifest,
        &f.set,
        &wrong_scope,
        &PlanLinkCredentials::default(),
        &AtomicBool::new(false)
    )
    .is_err());
    assert!(!f.dir.path().join("linked-0.pdf").exists());
}

#[test]
fn cancellation_after_first_commit_retains_its_receipt_and_remaining_file() {
    let engine = engine();
    let f = fixture(&engine);
    let plan = prepare(&engine, &f, &review(&engine, &f));
    let cancel = AtomicBool::new(false);
    let result = publish_plan_link_batch(
        plan,
        &engine,
        &PlanLinkCredentials::default(),
        &cancel,
        |progress| {
            assert_eq!(progress.completed, 1);
            cancel.store(true, Ordering::Relaxed);
        },
    );
    assert_eq!(result.status, PlanLinkBatchStatus::Cancelled);
    assert_eq!(result.completed.len(), 1);
    assert_eq!(result.remaining.len(), 1);
    assert!(result.completed[0].output.exists());
    assert!(!result.remaining[0].output.exists());
    assert_eq!(result.completed[0].report.created.len(), 1);
}

#[test]
fn late_output_collision_keeps_first_receipt_and_never_overwrites_second() {
    let engine = engine();
    let f = fixture(&engine);
    let plan = prepare(&engine, &f, &review(&engine, &f));
    let collision = plan.files()[1].output.clone();
    let result = publish_plan_link_batch(
        plan,
        &engine,
        &PlanLinkCredentials::default(),
        &AtomicBool::new(false),
        |_| std::fs::write(&collision, b"another writer").unwrap(),
    );
    assert_eq!(result.status, PlanLinkBatchStatus::Failed);
    assert_eq!(result.completed.len(), 1);
    assert_eq!(result.remaining.len(), 1);
    assert_eq!(std::fs::read(collision).unwrap(), b"another writer");
    assert!(result.completed[0].output.exists());
}

#[test]
fn future_source_or_manifest_changes_keep_committed_receipts_and_fail_aggregate_validation() {
    let engine = engine();
    for manifest in [false, true] {
        let f = fixture(&engine);
        let plan = prepare(&engine, &f, &review(&engine, &f));
        let changed = if manifest {
            f.manifest.clone()
        } else {
            f.dir.path().join("B.pdf")
        };
        let result = publish_plan_link_batch(
            plan,
            &engine,
            &PlanLinkCredentials::default(),
            &AtomicBool::new(false),
            |_| std::fs::write(&changed, b"changed source identity").unwrap(),
        );
        assert_eq!(result.status, PlanLinkBatchStatus::Failed);
        assert_eq!(result.completed.len(), 1);
        assert_eq!(result.remaining.len(), 1);
        assert!(!result.last_input_validation.as_ref().unwrap().valid);
        assert!(result.completed[0].output.exists());
        assert!(!result.remaining[0].output.exists());
    }
}

#[test]
fn final_input_change_reports_failure_even_with_all_outputs_saved() {
    let engine = engine();
    let f = fixture(&engine);
    let plan = prepare(&engine, &f, &review(&engine, &f));
    let result = publish_plan_link_batch(
        plan,
        &engine,
        &PlanLinkCredentials::default(),
        &AtomicBool::new(false),
        |progress| {
            if progress.completed == progress.total {
                std::fs::write(&f.manifest, b"changed").unwrap();
            }
        },
    );
    assert_eq!(result.status, PlanLinkBatchStatus::Failed);
    assert_eq!(result.completed.len(), 2);
    assert!(result.remaining.is_empty());
    assert!(result.failure.as_ref().unwrap().source_file_id.is_none());
    assert!(!result.last_input_validation.as_ref().unwrap().valid);
    assert!(result.completed.iter().all(|r| r.output.exists()));
}

#[test]
fn changed_input_before_first_commit_does_not_write_any_copy() {
    let engine = engine();
    let f = fixture(&engine);
    let plan = prepare(&engine, &f, &review(&engine, &f));
    std::fs::write(f.dir.path().join("B.pdf"), pdf("modified")).unwrap();
    let result = publish_plan_link_batch(
        plan,
        &engine,
        &PlanLinkCredentials::default(),
        &AtomicBool::new(false),
        |_| panic!("no commit expected"),
    );
    assert_eq!(result.status, PlanLinkBatchStatus::Failed);
    assert!(result.completed.is_empty());
    assert_eq!(result.remaining.len(), 2);
    assert!(result.remaining.iter().all(|f| !f.output.exists()));
}

#[test]
fn strict_batch_envelopes_reject_duplicates_unknown_keys_and_page_and_byte_limits() {
    let source = PlanLinkScanRequest {
        source_file_id: "source".into(),
        pages: vec![1],
        target_sheet_ids: vec!["target".into()],
    };
    let parse_request = |sources| {
        parse_batch_scan_request(
            &serde_json::to_vec(&PlanLinkBatchScanRequest { sources }).unwrap(),
        )
    };
    assert!(parse_request(vec![source.clone()]).is_ok());
    assert!(parse_request(vec![source.clone(), source.clone()]).is_err());
    let mut many: Vec<_> = (0..17)
        .map(|i| {
            let mut s = source.clone();
            s.source_file_id = format!("file-{i}");
            s
        })
        .collect();
    assert!(parse_request(many.clone()).is_err());
    many.truncate(5);
    for s in &mut many {
        s.pages = (1..=32).collect();
    }
    assert!(parse_request(many).is_err());
    assert!(parse_batch_scan_request(br#"{"sources":[],"surprise":true}"#).is_err());
    assert!(parse_batch_scan_request(&vec![b' '; MAX_BATCH_RECIPE_BYTES + 1]).is_err());
    assert!(parse_batch_review(&vec![b' '; MAX_BATCH_RECIPE_BYTES + 1]).is_err());
}

#[test]
fn explicit_ambiguous_destination_can_be_published_in_a_batch() {
    let engine = engine();
    let mut f = fixture(&engine);
    let other = f.dir.path().join("alternate.pdf");
    std::fs::write(&other, pdf("Alternate target")).unwrap();
    let ids = add_pdf(
        &mut f.set,
        &engine,
        &f.manifest,
        &other,
        None,
        &AddOptions {
            discipline: "Structure".into(),
            revision: String::new(),
        },
    )
    .unwrap();
    f.set
        .sheets
        .iter_mut()
        .find(|s| s.id == ids[0])
        .unwrap()
        .number = "B-20".into();
    f.request.sources[0].target_sheet_ids.extend(ids);
    std::fs::write(&f.manifest, encode(&f.set).unwrap()).unwrap();
    let inspection = inspect_plan_link_batch(
        &engine,
        &f.manifest,
        &f.set,
        &f.request,
        &PlanLinkCredentials::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(inspection.scan_complete);
    assert!(!inspection.complete());
    assert_eq!(inspection.files[0].occurrences[0].status, "ambiguous");
    let review = reviewed(&f, inspection);
    let result = publish_plan_link_batch(
        prepare(&engine, &f, &review),
        &engine,
        &PlanLinkCredentials::default(),
        &AtomicBool::new(false),
        |_| {},
    );
    assert!(result.complete(), "{result:?}");
    assert_eq!(
        result.completed[0].report.created[0].target.relative_path,
        "B.pdf"
    );
}

#[test]
fn protected_batch_preserves_encryption_and_requires_all_passwords_and_permissions() {
    let engine = engine();
    let cancel = AtomicBool::new(false);
    for allowed in [false, true] {
        let protection = Protection {
            user_password: "batch-fixture-reader".into(),
            owner_password: "batch-fixture-owner".into(),
            allow_annotate: allowed,
            ..Default::default()
        };
        let encrypted = security::protect(&pdf("B-20"), None, &protection).unwrap();
        let f = fixture_source(&engine, Some(&encrypted), Some(&protection.owner_password));
        let missing = inspect_plan_link_batch(
            &engine,
            &f.manifest,
            &f.set,
            &f.request,
            &PlanLinkCredentials::default(),
            &cancel,
        )
        .unwrap();
        assert!(!missing.complete());
        assert!(missing.files.is_empty());
        let passwords = [(
            f.request.sources[0].source_file_id.as_str(),
            protection.user_password.as_str(),
        )];
        let credentials = PlanLinkCredentials {
            passwords: &passwords,
        };
        let inspected = inspect_plan_link_batch(
            &engine,
            &f.manifest,
            &f.set,
            &f.request,
            &credentials,
            &cancel,
        )
        .unwrap();
        if !allowed {
            assert!(!inspected.complete());
            assert!(inspected.files.is_empty());
            continue;
        }
        assert!(inspected.complete());
        let review = reviewed(&f, inspected);
        assert!(prepare_plan_link_batch(
            &engine,
            &f.manifest,
            &f.set,
            &review,
            &PlanLinkCredentials::default(),
            &cancel
        )
        .is_err());
        let plan =
            prepare_plan_link_batch(&engine, &f.manifest, &f.set, &review, &credentials, &cancel)
                .unwrap();
        let result = publish_plan_link_batch(plan, &engine, &credentials, &cancel, |_| {});
        assert!(result.complete(), "{result:?}");
        let output = &result.completed[0].output;
        assert!(Pdf::load(output).unwrap().is_encrypted());
        assert!(engine.open(output, None).is_err());
        assert_eq!(
            engine
                .open(output, Some(&protection.user_password))
                .unwrap()
                .page_links(0)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            std::fs::read(f.dir.path().join("A.pdf")).unwrap(),
            encrypted
        );
    }
}

#[test]
fn signed_source_is_explicitly_stopped_without_writing_outputs() {
    let engine = engine();
    let mut signed = Pdf::load_mem(&pdf("B-20")).unwrap();
    signed.add_object(dictionary! {"Type"=>"Sig", "Filter"=>"Adobe.PPKLite"});
    let mut bytes = Vec::new();
    signed.save_to(&mut bytes).unwrap();
    let f = fixture_source(&engine, Some(&bytes), None);
    let report = inspect_plan_link_batch(
        &engine,
        &f.manifest,
        &f.set,
        &f.request,
        &PlanLinkCredentials::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(!report.complete());
    assert!(report.files.is_empty());
    assert_eq!(report.remaining_sources.len(), 2);
    assert!(report.stop_reason.unwrap().contains("signed"));
    assert!(!f.dir.path().join("linked-0.pdf").exists());
}

#[test]
fn occurrence_budget_stops_before_the_ninth_source_and_preserves_the_first_eight() {
    let engine = engine();
    let dir = tempfile::tempdir().unwrap();
    let manifest = dir.path().join("set.json");
    let mut set = PlanSet::new("Occurrence budget").unwrap();
    let target = dir.path().join("target.pdf");
    std::fs::write(&target, pdf("Target drawing")).unwrap();
    let targets = add_pdf(
        &mut set,
        &engine,
        &manifest,
        &target,
        None,
        &AddOptions::default(),
    )
    .unwrap();
    set.sheets
        .iter_mut()
        .find(|s| s.id == targets[0])
        .unwrap()
        .number = "T-00".into();
    let mut scans = Vec::new();
    for i in 0..9 {
        let path = dir.path().join(format!("source-{i}.pdf"));
        // Each token occupies a distinct text line so PDFium retains all occurrences.
        let mut document = Pdf::load_mem(&pdf("seed")).unwrap();
        let page_id = *document.get_pages().values().next().unwrap();
        let mut content = String::from("BT /F1 1 Tf ");
        for n in 0..512 {
            content.push_str(&format!("1 0 0 1 50 {} Tm (T-00) Tj ", 700.0 - n as f32));
        }
        content.push_str("ET");
        let stream = document.add_object(Stream::new(dictionary! {}, content.into_bytes()));
        document
            .get_object_mut(page_id)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Contents", stream);
        let mut bytes = Vec::new();
        document.save_to(&mut bytes).unwrap();
        std::fs::write(&path, bytes).unwrap();
        let ids = add_pdf(
            &mut set,
            &engine,
            &manifest,
            &path,
            None,
            &AddOptions::default(),
        )
        .unwrap();
        let sheet = set.sheets.iter_mut().find(|s| s.id == ids[0]).unwrap();
        sheet.number = format!("S-{i:02}");
        scans.push(PlanLinkScanRequest {
            source_file_id: sheet.file_id.clone(),
            pages: vec![1],
            target_sheet_ids: targets.clone(),
        });
    }
    std::fs::write(&manifest, encode(&set).unwrap()).unwrap();
    let request = PlanLinkBatchScanRequest { sources: scans };
    let report = inspect_plan_link_batch(
        &engine,
        &manifest,
        &set,
        &request,
        &PlanLinkCredentials::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(report.occurrences, MAX_BATCH_OCCURRENCES);
    assert_eq!(report.files.len(), 8);
    assert_eq!(report.scanned_pages, 8);
    assert_eq!(
        report.remaining_sources,
        [request.sources[8].source_file_id.clone()]
    );
    assert_eq!(
        report.stop_reason.as_deref(),
        Some("batch_occurrence_limit")
    );
    assert!(!report.scan_complete);
}
