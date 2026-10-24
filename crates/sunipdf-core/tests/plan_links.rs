use lopdf::{dictionary, Document as Pdf, Object, Stream};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex, MutexGuard, OnceLock,
};
use sunipdf_core::{
    plan_links::*, plan_set::*, security, Engine, EngineError, LinkTarget, Protection,
};

static SERIAL: Mutex<()> = Mutex::new(());
static ENGINE: OnceLock<EnginePtr> = OnceLock::new();
struct EnginePtr(*const Engine);
// Every access is serialized and PDFium remains initialized for this test process.
unsafe impl Send for EnginePtr {}
unsafe impl Sync for EnginePtr {}

struct SharedEngine {
    _guard: MutexGuard<'static, ()>,
    engine: &'static Engine,
}

impl std::ops::Deref for SharedEngine {
    type Target = Engine;
    fn deref(&self) -> &Self::Target {
        self.engine
    }
}

fn engine() -> SharedEngine {
    let guard = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
    let pointer = ENGINE.get_or_init(|| {
        EnginePtr(Box::leak(Box::new(
            Engine::new().expect("PDFium is required"),
        )))
    });
    SharedEngine {
        _guard: guard,
        engine: unsafe { &*pointer.0 },
    }
}

fn pdf(text: &str, rotation: i64) -> Vec<u8> {
    let mut pdf = Pdf::with_version("1.7");
    let pages = pdf.new_object_id();
    let font =
        pdf.add_object(dictionary! {"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"Helvetica"});
    let content = pdf.add_object(Stream::new(
        dictionary! {},
        format!("BT /F1 12 Tf 50 200 Td ({text}) Tj ET").into_bytes(),
    ));
    let note=pdf.add_object(dictionary!{"Type"=>"Annot","Subtype"=>"Text","Rect"=>vec![10.into(),10.into(),30.into(),30.into()],"Contents"=>Object::string_literal("Keep this note")});
    let page=pdf.add_object(dictionary!{"Type"=>"Page","Parent"=>pages,"MediaBox"=>vec![0.into(),0.into(),600.into(),800.into()],
        "CropBox"=>vec![20.into(),30.into(),580.into(),760.into()],"Rotate"=>rotation,
        "Resources"=>dictionary!{"Font"=>dictionary!{"F1"=>font}},"Contents"=>content,"Annots"=>vec![Object::Reference(note)]});
    pdf.objects.insert(
        pages,
        dictionary! {"Type"=>"Pages","Count"=>1,"Kids"=>vec![Object::Reference(page)]}.into(),
    );
    let catalog = pdf.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
    pdf.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}
struct Fixture {
    dir: tempfile::TempDir,
    manifest: PathBuf,
    set: PlanSet,
    scan: PlanLinkScanRequest,
    source: PathBuf,
    target: PathBuf,
}
fn fixture(
    engine: &Engine,
    rotation: i64,
    source_bytes: Option<Vec<u8>>,
    password: Option<&str>,
) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("source")).unwrap();
    std::fs::create_dir(dir.path().join("targets")).unwrap();
    let source = dir.path().join("source/Index.pdf");
    let target = dir.path().join("targets/Étage.pdf");
    std::fs::write(
        &source,
        source_bytes.unwrap_or_else(|| pdf("A-10 A-101 XA-10 A-10.1 A-10_OLD A-10", rotation)),
    )
    .unwrap();
    std::fs::write(&target, pdf("Target drawing", 0)).unwrap();
    let manifest = dir.path().join("set.json");
    let mut set = PlanSet::new("References").unwrap();
    let a = add_pdf(
        &mut set,
        engine,
        &manifest,
        &source,
        password,
        &AddOptions::default(),
    )
    .unwrap();
    let b = add_pdf(
        &mut set,
        engine,
        &manifest,
        &target,
        None,
        &AddOptions::default(),
    )
    .unwrap();
    set.sheets.iter_mut().find(|s| s.id == b[0]).unwrap().number = "A-10".into();
    let source_file_id = set
        .sheets
        .iter()
        .find(|s| s.id == a[0])
        .unwrap()
        .file_id
        .clone();
    let scan = PlanLinkScanRequest {
        source_file_id,
        pages: vec![1],
        target_sheet_ids: b,
    };
    std::fs::write(&manifest, encode(&set).unwrap()).unwrap();
    Fixture {
        dir,
        manifest,
        set,
        scan,
        source,
        target,
    }
}
fn review(inspection: &PlanLinkInspection, scan: &PlanLinkScanRequest) -> PlanLinkReview {
    PlanLinkReview {
        version: 1,
        manifest_sha256: inspection.manifest_sha256.clone(),
        source_sha256: inspection.source_sha256.clone(),
        scan: scan.clone(),
        selections: inspection
            .occurrences
            .iter()
            .filter(|o| o.status == "ready" || o.status == "ambiguous")
            .map(|o| PlanLinkSelection {
                occurrence_id: o.id.clone(),
                fingerprint: o.fingerprint.clone(),
                target_sheet_id: o.target_sheet_ids[0].clone(),
                confirmed: true,
            })
            .collect(),
    }
}
fn inspect(
    engine: &Engine,
    f: &Fixture,
    credentials: &PlanLinkCredentials<'_>,
) -> PlanLinkInspection {
    inspect_plan_links(
        engine,
        &f.manifest,
        &f.set,
        &f.scan,
        credentials,
        &AtomicBool::new(false),
    )
    .unwrap()
}
#[test]
fn exact_references_round_trip_rotated_pages_and_confined_sibling_targets() {
    let engine = engine();
    let credentials = PlanLinkCredentials::default();
    let cancel = AtomicBool::new(false);
    for rotation in [0, 90, 180, 270] {
        let f = fixture(&engine, rotation, None, None);
        let original = std::fs::read(&f.source).unwrap();
        let target_original = std::fs::read(&f.target).unwrap();
        let inspection = inspect(&engine, &f, &credentials);
        assert!(inspection.complete(), "{:?}", inspection.occurrences);
        assert_eq!(inspection.occurrences.len(), 2);
        assert!(inspection.occurrences.iter().all(|o| o.text == "A-10"));
        let request = review(&inspection, &f.scan);
        assert!(parse_review(&encode_review(&request).unwrap()).is_ok());
        let prepared = prepare_plan_links(
            &engine,
            &f.manifest,
            &f.set,
            &request,
            &credentials,
            &cancel,
        )
        .unwrap();
        let image = prepared
            .render_preview(&engine, 1, None, 512, &cancel)
            .unwrap();
        assert!(image.width <= 512 && image.height <= 512);
        let target_image = prepared
            .render_target_preview(&engine, &f.scan.target_sheet_ids[0], None, 512, &cancel)
            .unwrap();
        assert!(target_image.width <= 512 && target_image.height <= 512);
        assert_ne!(target_image.rgba, image.rgba);
        assert!(prepared
            .render_target_preview(&engine, "unreviewed-sheet", None, 512, &cancel)
            .is_err());
        assert!(prepared
            .render_target_preview(&engine, &f.scan.target_sheet_ids[0], None, 1537, &cancel)
            .is_err());
        assert_eq!(prepared.report().created.len(), 2);
        let output = f.source.with_file_name("Index-linked.pdf");
        let receipt = publish_plan_links(prepared, &output, &cancel).unwrap();
        assert!(receipt.report.complete());
        let doc = engine.open(&output, None).unwrap();
        let links = doc.page_links(0).unwrap();
        assert_eq!(links.len(), 2);
        let LinkTarget::RemotePage(target) = &links[0].target else {
            panic!("remote link was not read")
        };
        assert_eq!(target.relative_path, "../targets/Étage.pdf");
        assert_eq!(target.page, 0);
        let verified = resolve_verified_remote_link(
            &engine,
            &f.manifest,
            &f.set,
            &output,
            target,
            &credentials,
            &cancel,
        )
        .unwrap();
        assert_eq!(verified.bytes, target_original);
        assert_eq!(verified.page, 1);
        let parsed = Pdf::load(&output).unwrap();
        let page = parsed.get_dictionary(parsed.get_pages()[&1]).unwrap();
        let annotations = page.get(b"Annots").unwrap().as_array().unwrap();
        assert_eq!(annotations.len(), 3);
        let note = parsed
            .get_dictionary(annotations[0].as_reference().unwrap())
            .unwrap();
        assert_eq!(
            note.get(b"Contents").unwrap().as_str().unwrap(),
            b"Keep this note"
        );
        assert_eq!(std::fs::read(&f.source).unwrap(), original);
        assert_eq!(std::fs::read(&f.target).unwrap(), target_original);
        assert_eq!(
            doc.page_text(0).unwrap(),
            engine.open(&f.source, None).unwrap().page_text(0).unwrap()
        );
    }
}
#[test]
fn explicit_ambiguity_review_preserves_unselected_occurrences() {
    let engine = engine();
    let mut f = fixture(&engine, 0, None, None);
    let credentials = PlanLinkCredentials::default();
    let cancel = AtomicBool::new(false);
    let other = f.dir.path().join("targets/Other.pdf");
    std::fs::write(&other, pdf("Another drawing", 0)).unwrap();
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
        .number = "A-10".into();
    f.scan.target_sheet_ids.extend(ids);
    std::fs::write(&f.manifest, encode(&f.set).unwrap()).unwrap();
    let inspection = inspect(&engine, &f, &credentials);
    assert!(!inspection.complete());
    assert!(inspection.scan_complete);
    assert!(inspection
        .occurrences
        .iter()
        .all(|o| o.status == "ambiguous"));
    let mut request = review(&inspection, &f.scan);
    request.selections.truncate(1);
    request.selections[0].confirmed = false;
    assert!(prepare_plan_links(
        &engine,
        &f.manifest,
        &f.set,
        &request,
        &credentials,
        &cancel
    )
    .is_err());
    request.selections[0].confirmed = true;
    let prepared = prepare_plan_links(
        &engine,
        &f.manifest,
        &f.set,
        &request,
        &credentials,
        &cancel,
    )
    .unwrap();
    assert_eq!(prepared.report().created.len(), 1);
    assert_eq!(prepared.report().excluded_occurrences.len(), 1);
    let mut forged = request;
    forged.selections[0].target_sheet_id = "not-indexed".into();
    assert!(
        prepare_plan_links(&engine, &f.manifest, &f.set, &forged, &credentials, &cancel).is_err()
    );
}
#[test]
fn stale_sources_manifest_output_collisions_and_cancel_publish_nothing() {
    let engine = engine();
    let credentials = PlanLinkCredentials::default();
    let cancel = AtomicBool::new(false);
    for kind in [
        "target",
        "source",
        "manifest",
        "collision",
        "cancel",
        "other-parent",
    ] {
        let f = fixture(&engine, 0, None, None);
        let inspection = inspect(&engine, &f, &credentials);
        let request = review(&inspection, &f.scan);
        let prepared = prepare_plan_links(
            &engine,
            &f.manifest,
            &f.set,
            &request,
            &credentials,
            &cancel,
        )
        .unwrap();
        let mut out = f.source.with_file_name("linked.pdf");
        let stop = AtomicBool::new(false);
        match kind {
            "target" => std::fs::write(&f.target, pdf("Changed drawing", 0)).unwrap(),
            "source" => std::fs::write(&f.source, pdf("Changed source", 0)).unwrap(),
            "manifest" => {
                let mut bytes = std::fs::read(&f.manifest).unwrap();
                bytes.push(b' ');
                std::fs::write(&f.manifest, bytes).unwrap();
            }
            "collision" => std::fs::write(&out, b"existing output").unwrap(),
            "cancel" => stop.store(true, Ordering::Relaxed),
            "other-parent" => out = f.dir.path().join("linked.pdf"),
            _ => unreachable!(),
        }
        assert!(publish_plan_links(prepared, &out, &stop).is_err(), "{kind}");
        if kind == "collision" {
            assert_eq!(std::fs::read(out).unwrap(), b"existing output");
        } else {
            assert!(!out.exists());
        }
    }
}
#[test]
fn protected_sources_preserve_encryption_and_restricted_sources_are_refused() {
    let engine = engine();
    let cancel = AtomicBool::new(false);
    for allowed in [false, true] {
        let protection = Protection {
            user_password: "plan-links-reader".into(),
            owner_password: "plan-links-owner".into(),
            allow_annotate: allowed,
            ..Default::default()
        };
        let bytes = security::protect(&pdf("A-10", 0), None, &protection).unwrap();
        let f = fixture(
            &engine,
            0,
            Some(bytes.clone()),
            Some(&protection.owner_password),
        );
        let passwords = [(
            f.scan.source_file_id.as_str(),
            protection.user_password.as_str(),
        )];
        let credentials = PlanLinkCredentials {
            passwords: &passwords,
        };
        let result =
            inspect_plan_links(&engine, &f.manifest, &f.set, &f.scan, &credentials, &cancel);
        if !allowed {
            assert!(matches!(result, Err(EngineError::SecurityRestricted)));
            continue;
        }
        let request = review(&result.unwrap(), &f.scan);
        let prepared = prepare_plan_links(
            &engine,
            &f.manifest,
            &f.set,
            &request,
            &credentials,
            &cancel,
        )
        .unwrap();
        let out = f.source.with_file_name("encrypted-linked.pdf");
        publish_plan_links(prepared, &out, &cancel).unwrap();
        assert!(Pdf::load(&out).unwrap().is_encrypted());
        assert!(engine.open(&out, None).is_err());
        assert_eq!(
            engine
                .open(&out, Some(&protection.user_password))
                .unwrap()
                .page_links(0)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(std::fs::read(&f.source).unwrap(), bytes);
    }
}
#[test]
fn existing_links_and_changed_destination_remain_explicit() {
    let engine = engine();
    let mut f = fixture(&engine, 0, None, None);
    let credentials = PlanLinkCredentials::default();
    let cancel = AtomicBool::new(false);
    let inspection = inspect(&engine, &f, &credentials);
    let request = review(&inspection, &f.scan);
    let prepared = prepare_plan_links(
        &engine,
        &f.manifest,
        &f.set,
        &request,
        &credentials,
        &cancel,
    )
    .unwrap();
    let out = f.source.with_file_name("linked.pdf");
    publish_plan_links(prepared, &out, &cancel).unwrap();
    let doc = engine.open(&out, None).unwrap();
    let links = doc.page_links(0).unwrap();
    let LinkTarget::RemotePage(target) = links[0].target.clone() else {
        panic!("missing target")
    };
    std::fs::write(&f.source, std::fs::read(&out).unwrap()).unwrap();
    refresh_exact(
        &mut f.set,
        &engine,
        &f.manifest,
        &f.scan.source_file_id,
        None,
        None,
    )
    .unwrap();
    std::fs::write(&f.manifest, encode(&f.set).unwrap()).unwrap();
    let scan = inspect(&engine, &f, &credentials);
    assert!(scan.occurrences.iter().all(|o| o.status == "existing_link"));
    std::fs::write(&f.target, pdf("Changed drawing", 0)).unwrap();
    assert!(resolve_verified_remote_link(
        &engine,
        &f.manifest,
        &f.set,
        &out,
        &target,
        &credentials,
        &cancel
    )
    .is_err());
}

#[test]
fn absent_text_missing_targets_and_scan_limits_are_reported_without_writes() {
    let engine = engine();
    let credentials = PlanLinkCredentials::default();
    let cancel = AtomicBool::new(false);
    let blank = fixture(&engine, 0, Some(pdf("", 0)), None);
    let report = inspect(&engine, &blank, &credentials);
    assert!(!report.complete());
    assert!(report.scan_complete);
    assert!(report.occurrences.is_empty());
    assert!(report
        .issues
        .iter()
        .any(|i| i.reason == "no_searchable_text"));
    let missing = fixture(&engine, 0, None, None);
    std::fs::rename(&missing.target, missing.target.with_extension("kept")).unwrap();
    let report = inspect(&engine, &missing, &credentials);
    assert_eq!(report.targets[0].status, "unavailable_reference");
    assert!(report
        .occurrences
        .iter()
        .all(|o| o.status == "unavailable_target"));
    assert!(!report.complete());

    let many = fixture(
        &engine,
        0,
        Some(pdf(&"A-10 ".repeat(MAX_OCCURRENCES + 1), 0)),
        None,
    );
    let original = std::fs::read(&many.source).unwrap();
    let report = inspect(&engine, &many, &credentials);
    assert!(report.truncated);
    assert!(!report.complete());
    assert!(!report.scan_complete);
    assert_eq!(report.occurrences.len(), MAX_OCCURRENCES);
    let mut request = many.scan.clone();
    request.pages = vec![1; MAX_SCAN_PAGES + 1];
    assert!(inspect_plan_links(
        &engine,
        &many.manifest,
        &many.set,
        &request,
        &credentials,
        &cancel
    )
    .is_err());
    assert_eq!(std::fs::read(&many.source).unwrap(), original);
    assert!(!many.source.with_file_name("linked.pdf").exists());
}

#[test]
fn signature_fields_and_copy_restricted_destinations_are_refused() {
    let engine = engine();
    let credentials = PlanLinkCredentials::default();
    let cancel = AtomicBool::new(false);
    let mut signed = Pdf::load_mem(&pdf("A-10", 0)).unwrap();
    let field =
        signed.add_object(dictionary! {"FT"=>"Sig","T"=>Object::string_literal("Approval")});
    signed.catalog_mut().unwrap().set(
        "AcroForm",
        dictionary! {"Fields"=>vec![Object::Reference(field)]},
    );
    let mut bytes = Vec::new();
    signed.save_to(&mut bytes).unwrap();
    let f = fixture(&engine, 0, Some(bytes.clone()), None);
    let err = inspect_plan_links(&engine, &f.manifest, &f.set, &f.scan, &credentials, &cancel)
        .unwrap_err();
    assert!(err.to_string().contains("signed source"));
    assert_eq!(std::fs::read(&f.source).unwrap(), bytes);

    let mut f = fixture(&engine, 0, None, None);
    let protection = Protection {
        user_password: "destination-reader".into(),
        owner_password: "destination-owner".into(),
        allow_copy: false,
        ..Default::default()
    };
    let protected =
        security::protect(&std::fs::read(&f.target).unwrap(), None, &protection).unwrap();
    std::fs::write(&f.target, &protected).unwrap();
    let target_file = f
        .set
        .sheets
        .iter()
        .find(|s| s.id == f.scan.target_sheet_ids[0])
        .unwrap()
        .file_id
        .clone();
    refresh_exact(
        &mut f.set,
        &engine,
        &f.manifest,
        &target_file,
        None,
        Some(&protection.owner_password),
    )
    .unwrap();
    std::fs::write(&f.manifest, encode(&f.set).unwrap()).unwrap();
    let passwords = [(target_file.as_str(), protection.user_password.as_str())];
    let credentials = PlanLinkCredentials {
        passwords: &passwords,
    };
    let report = inspect(&engine, &f, &credentials);
    assert_eq!(report.targets[0].status, "copy_restricted");
    assert!(report
        .occurrences
        .iter()
        .all(|o| o.status == "unavailable_target"));
    assert!(!report.complete());
    assert_eq!(std::fs::read(&f.target).unwrap(), protected);
}

#[test]
fn authored_links_remain_portable_when_the_whole_set_is_copied() {
    let engine = engine();
    let f = fixture(&engine, 0, None, None);
    let credentials = PlanLinkCredentials::default();
    let cancel = AtomicBool::new(false);
    let report = inspect(&engine, &f, &credentials);
    let prepared = prepare_plan_links(
        &engine,
        &f.manifest,
        &f.set,
        &review(&report, &f.scan),
        &credentials,
        &cancel,
    )
    .unwrap();
    let out = f.source.with_file_name("linked.pdf");
    let receipt = publish_plan_links(prepared, &out, &cancel).unwrap();
    let copy = tempfile::tempdir().unwrap();
    std::fs::create_dir(copy.path().join("source")).unwrap();
    std::fs::create_dir(copy.path().join("targets")).unwrap();
    for source in [&f.source, &f.target, &f.manifest, &out] {
        std::fs::copy(
            source,
            copy.path().join(source.strip_prefix(f.dir.path()).unwrap()),
        )
        .unwrap();
    }
    let source = copy.path().join("source/linked.pdf");
    let verified = resolve_verified_remote_link(
        &engine,
        &copy.path().join("set.json"),
        &f.set,
        &source,
        &receipt.report.created[0].target,
        &credentials,
        &cancel,
    )
    .unwrap();
    assert_eq!(verified.bytes, std::fs::read(&f.target).unwrap());
    assert!(verified
        .path
        .starts_with(copy.path().canonicalize().unwrap()));
}

#[cfg(any(unix, windows))]
#[test]
fn replacing_a_checked_target_directory_with_an_alias_invalidates_the_candidate() {
    let engine = engine();
    let f = fixture(&engine, 0, None, None);
    let credentials = PlanLinkCredentials::default();
    let cancel = AtomicBool::new(false);
    let report = inspect(&engine, &f, &credentials);
    let prepared = prepare_plan_links(
        &engine,
        &f.manifest,
        &f.set,
        &review(&report, &f.scan),
        &credentials,
        &cancel,
    )
    .unwrap();
    let selected = f.dir.path().join("targets");
    let retained = f.dir.path().join("retained-targets");
    std::fs::rename(&selected, &retained).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&retained, &selected).unwrap();
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let result = std::process::Command::new("cmd.exe")
            .creation_flags(0x0800_0000)
            .args(["/D", "/C", "mklink", "/J"])
            .arg(&selected)
            .arg(&retained)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let out = f.source.with_file_name("linked.pdf");
    assert!(prepared.revalidate_sources(&cancel).is_err());
    assert!(publish_plan_links(prepared, &out, &cancel).is_err());
    assert!(!out.exists());
    #[cfg(windows)]
    std::fs::remove_dir(selected).unwrap();
    #[cfg(unix)]
    std::fs::remove_file(selected).unwrap();
}
