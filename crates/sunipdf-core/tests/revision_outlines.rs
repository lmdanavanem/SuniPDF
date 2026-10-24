use lopdf::{dictionary, Document as Pdf, Object, Stream};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex, MutexGuard, OnceLock,
};
use sunipdf_core::{
    revision_outlines::*,
    revision_overlay::{OverlayAlignment, OverlayTransform},
    security, AnnotShape, AnnotStyle, Engine, EngineError, PointRect, Protection,
};

static SERIAL: Mutex<()> = Mutex::new(());
static ENGINE: OnceLock<EnginePtr> = OnceLock::new();
struct EnginePtr(*const Engine);
// Access is serialized and the one engine is retained for this test process.
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
    let ptr = ENGINE.get_or_init(|| {
        EnginePtr(Box::leak(Box::new(
            Engine::new().expect("PDFium is required"),
        )))
    });
    SharedEngine {
        _guard: guard,
        engine: unsafe { &*ptr.0 },
    }
}
fn fixture(rotation: i64, content: &str) -> Vec<u8> {
    let mut pdf = Pdf::with_version("1.7");
    let pages = pdf.new_object_id();
    let stream = pdf.add_object(Stream::new(dictionary! {}, content.as_bytes().to_vec()));
    let page = pdf.add_object(dictionary! {"Type"=>"Page","Parent"=>pages,
    "MediaBox"=>vec![0.into(),0.into(),500.into(),500.into()],
    "CropBox"=>vec![100.into(),50.into(),300.into(),250.into()],"Rotate"=>rotation,
    "Resources"=>dictionary!{},"Contents"=>stream});
    pdf.objects.insert(
        pages,
        dictionary! {"Type"=>"Pages","Kids"=>vec![page.into()],"Count"=>1}.into(),
    );
    let root = pdf.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
    pdf.trailer.set("Root", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}
fn files(dir: &Path, rotation: i64) -> (PathBuf, PathBuf) {
    let old = dir.join("old.pdf");
    let new = dir.join("new.pdf");
    std::fs::write(&old, fixture(rotation, "q Q")).unwrap();
    std::fs::write(
        &new,
        fixture(rotation, "0 g 130 90 20 30 re f 220 180 20 20 re f"),
    )
    .unwrap();
    (old, new)
}
fn options() -> OutlineOptions {
    OutlineOptions {
        long_side: 200,
        min_component_pixels: 1,
        merge_gap_pixels: 0,
        ..Default::default()
    }
}
fn inspect(engine: &Engine, old: &Path, new: &Path) -> OutlineInspection {
    inspect_revision_outlines(
        engine,
        old,
        new,
        &options(),
        OutlinePasswords::default(),
        &AtomicBool::new(false),
        |_| {},
    )
    .unwrap()
}
fn review(inspection: &OutlineInspection, class: OutlineClassification) -> OutlineReview {
    OutlineReview {
        version: 1,
        old_sha256: inspection.old_sha256.clone(),
        new_sha256: inspection.new_sha256.clone(),
        options: inspection.options.clone(),
        selections: inspection
            .regions
            .iter()
            .map(|r| OutlineSelection {
                id: r.id.clone(),
                fingerprint: r.fingerprint.clone(),
                classification: class,
                confirmed: true,
            })
            .collect(),
    }
}
fn prepare(
    engine: &Engine,
    old: &Path,
    new: &Path,
    review: &OutlineReview,
) -> PreparedRevisionOutlines {
    prepare_revision_outlines(
        engine,
        old,
        new,
        review,
        OutlinePasswords::default(),
        &AtomicBool::new(false),
        |_| {},
    )
    .unwrap()
}

#[test]
fn native_outlines_preserve_sources_target_annotations_and_provenance_across_rotations() {
    let engine = engine();
    let cancel = AtomicBool::new(false);
    for rotation in [0, 90, 180, 270] {
        let dir = tempfile::tempdir().unwrap();
        let (old, new) = files(dir.path(), rotation);
        let mut target = engine.open(&new, None).unwrap();
        target
            .add_shape_annotation(
                0,
                AnnotShape::Square,
                PointRect {
                    left: 80.0,
                    bottom: 80.0,
                    right: 95.0,
                    top: 95.0,
                },
                &AnnotStyle::default(),
                Some("Existing reviewer"),
            )
            .unwrap();
        target.save_as(&new).unwrap();
        drop(target);
        let old_bytes = std::fs::read(&old).unwrap();
        let new_bytes = std::fs::read(&new).unwrap();
        let inspection = inspect(&engine, &old, &new);
        assert!(inspection.complete());
        assert_eq!(inspection.new_geometry.rotation_degrees, rotation as u16);
        assert!(inspection.regions.len() >= 2);
        let mut request = review(&inspection, OutlineClassification::ContentChange);
        request.selections[1].classification = OutlineClassification::Uncertain;
        let prepared = prepare(&engine, &old, &new, &request);
        assert_eq!(prepared.report().created.len(), inspection.regions.len());
        let image = prepared
            .render_preview(&engine, None, 200, &cancel)
            .unwrap();
        assert_ne!(image.rgba, inspection.images.as_ref().unwrap().new.rgba);
        let output = dir.path().join("reviewed.pdf");
        let report = publish_revision_outlines(prepared, &output, &cancel).unwrap();
        assert!(report.complete());
        let mut reopened = engine.open(&output, None).unwrap();
        assert_eq!(reopened.page_count(), 1);
        let records = reopened.revision_outline_records(1).unwrap();
        assert_eq!(records.len(), report.created.len());
        assert!(records.iter().all(|r| r.geometry_matches_original
            && r.trace.old_sha256 == inspection.old_sha256
            && r.trace.new_sha256 == inspection.new_sha256));
        let register = reopened.annotation_register().unwrap();
        assert!(register
            .rows
            .iter()
            .any(|r| r.author.as_deref() == Some("Existing reviewer")));
        assert!(register
            .rows
            .iter()
            .any(|r| r.fields.business_status.as_deref() == Some("Uncertain")));
        let captures = reopened.library_capture_candidates(0).unwrap();
        assert!(report.created.iter().all(|c| captures[c.index]
            .refusal
            .as_ref()
            .unwrap()
            .contains("provenance")));
        let first = &report.created[0];
        reopened.move_annotation(0, first.index, 1.0, 1.0).unwrap();
        reopened
            .set_annotation_contents(0, first.index, "Reviewed after site inspection")
            .unwrap();
        reopened.save_as(&output).unwrap();
        let edited = engine.open(&output, None).unwrap();
        let records = edited.revision_outline_records(1).unwrap();
        assert!(
            !records
                .iter()
                .find(|r| r.index == first.index)
                .unwrap()
                .geometry_matches_original
        );
        assert_eq!(std::fs::read(&old).unwrap(), old_bytes);
        assert_eq!(std::fs::read(&new).unwrap(), new_bytes);
        let raw = Pdf::load(&output).unwrap();
        assert!(raw.objects.values().any(|o| o.as_dict().is_ok_and(|d| d
            .get(b"IT")
            .and_then(Object::as_name)
            .ok()
            == Some(b"PolygonCloud".as_slice())
            && d.has(b"AP")
            && d.has(b"SuniPDFRevisionOutline"))));
    }
}

#[test]
fn identical_noise_only_and_partial_decisions_never_claim_unreviewed_changes() {
    let engine = engine();
    let dir = tempfile::tempdir().unwrap();
    let (old, new) = files(dir.path(), 0);
    let cancel = AtomicBool::new(false);
    let identical = inspect(&engine, &old, &old);
    assert!(identical.complete());
    assert_eq!(identical.changed_pixels, 0);
    assert!(identical.regions.is_empty());
    let inspection = inspect(&engine, &old, &new);
    assert_eq!(inspection.regions.len(), 2);
    let noise = prepare(
        &engine,
        &old,
        &new,
        &review(&inspection, OutlineClassification::RenderingNoise),
    );
    assert!(noise.report().complete());
    assert!(!noise.report().can_publish());
    assert_eq!(
        noise
            .render_preview(&engine, None, 200, &cancel)
            .unwrap()
            .rgba,
        inspection.images.as_ref().unwrap().new.rgba
    );
    let out = dir.path().join("noise.pdf");
    assert!(publish_revision_outlines(noise, &out, &cancel).is_err());
    assert!(!out.exists());
    let mut partial = review(&inspection, OutlineClassification::ContentChange);
    partial.selections.pop();
    let candidate = prepare(&engine, &old, &new, &partial);
    assert!(!candidate.report().complete());
    assert_eq!(candidate.report().excluded_regions.len(), 1);
    assert!(
        publish_revision_outlines(candidate, &dir.path().join("partial.pdf"), &cancel)
            .unwrap()
            .can_publish()
    );
}

#[test]
fn reviews_require_exact_sources_options_confirmations_and_new_outputs() {
    let engine = engine();
    let dir = tempfile::tempdir().unwrap();
    let (old, new) = files(dir.path(), 0);
    let cancel = AtomicBool::new(false);
    let original_old = std::fs::read(&old).unwrap();
    let original_new = std::fs::read(&new).unwrap();
    let inspection = inspect(&engine, &old, &new);
    let request = review(&inspection, OutlineClassification::ContentChange);
    for mode in 0..5 {
        let mut bad = request.clone();
        match mode {
            0 => bad.selections[0].confirmed = false,
            1 => bad.options.channel_tolerance += 1,
            2 => bad.selections[0].fingerprint = "0".repeat(64),
            3 => bad.selections.push(bad.selections[0].clone()),
            _ => bad.version = 2,
        }
        assert!(prepare_revision_outlines(
            &engine,
            &old,
            &new,
            &bad,
            OutlinePasswords::default(),
            &cancel,
            |_| {}
        )
        .is_err());
    }
    assert!(parse_review(&vec![b' '; 256 * 1024 + 1]).is_err());
    assert!(parse_review(&encode_review(&request).unwrap()).is_ok());
    let candidate = prepare(&engine, &old, &new, &request);
    assert!(matches!(
        publish_revision_outlines(candidate, &new, &cancel),
        Err(EngineError::AlreadyExists(_))
    ));
    let candidate = prepare(&engine, &old, &new, &request);
    let mut changed = original_new.clone();
    changed.extend_from_slice(b"\n% changed saved revision\n");
    std::fs::write(&new, &changed).unwrap();
    let output = dir.path().join("stale.pdf");
    assert!(candidate
        .render_preview(&engine, None, 200, &cancel)
        .is_err());
    assert!(publish_revision_outlines(candidate, &output, &cancel).is_err());
    assert!(!output.exists());
    std::fs::write(&new, &original_new).unwrap();
    let candidate = prepare(&engine, &old, &new, &request);
    cancel.store(true, Ordering::Relaxed);
    assert!(matches!(
        publish_revision_outlines(candidate, &output, &cancel),
        Err(EngineError::Cancelled)
    ));
    assert!(!output.exists());
    assert_eq!(std::fs::read(&old).unwrap(), original_old);
    assert_eq!(std::fs::read(&new).unwrap(), original_new);
}

#[test]
fn alignment_coverage_edges_and_progress_are_explicit() {
    let engine = engine();
    let dir = tempfile::tempdir().unwrap();
    let (old, new) = files(dir.path(), 0);
    let cancel = AtomicBool::new(false);
    let mut phases = Vec::new();
    let options = OutlineOptions {
        alignment: OverlayAlignment::Manual {
            transform: OverlayTransform {
                translation: [20.0, 0.0],
                ..Default::default()
            },
        },
        ..options()
    };
    let inspection = inspect_revision_outlines(
        &engine,
        &old,
        &new,
        &options,
        OutlinePasswords::default(),
        &cancel,
        |p| phases.push((p.phase, p.done, p.total)),
    )
    .unwrap();
    assert!(!inspection.complete());
    assert!(inspection.unmapped_pixels > 0);
    assert!(inspection.unmapped_old_pixels > 0);
    assert!(phases.iter().any(|p| p.0 == "compare" && p.1 == p.2));
    assert!(phases.iter().any(|p| p.0 == "components" && p.1 == p.2));
    let options = OutlineOptions {
        alignment: OverlayAlignment::TwoPoints {
            base: [[20.0, 0.0], [120.0, 0.0]],
            revision: [[0.0, 0.0], [100.0, 0.0]],
        },
        ..options
    };
    let anchored = inspect_revision_outlines(
        &engine,
        &old,
        &new,
        &options,
        OutlinePasswords::default(),
        &cancel,
        |_| {},
    )
    .unwrap();
    assert_eq!(anchored.transform, inspection.transform);
    assert_eq!(anchored.changed_pixels, inspection.changed_pixels);
    std::fs::write(&new, fixture(0, "0 g 100 50 10 10 re f")).unwrap();
    let edge = inspect(&engine, &old, &new);
    assert_eq!(edge.regions[0].status, "outline_out_of_bounds");
    assert!(prepare_revision_outlines(
        &engine,
        &old,
        &new,
        &review(&edge, OutlineClassification::ContentChange),
        OutlinePasswords::default(),
        &cancel,
        |_| {}
    )
    .is_err());
    assert!(prepare(
        &engine,
        &old,
        &new,
        &review(&edge, OutlineClassification::RenderingNoise)
    )
    .report()
    .complete());
    let cancelled_inspection = inspect_revision_outlines(
        &engine,
        &old,
        &new,
        &options,
        OutlinePasswords::default(),
        &cancel,
        |p| {
            if p.phase == "compare" {
                cancel.store(true, Ordering::Relaxed);
            }
        },
    );
    assert!(matches!(cancelled_inspection, Err(EngineError::Cancelled)));
}

#[test]
fn protected_and_signed_targets_keep_safety_boundaries() {
    let engine = engine();
    let dir = tempfile::tempdir().unwrap();
    let (old, new) = files(dir.path(), 0);
    let cancel = AtomicBool::new(false);
    let old_raw = std::fs::read(&old).unwrap();
    let new_raw = std::fs::read(&new).unwrap();
    let old_secret = format!("old-{}", dir.path().file_name().unwrap().to_string_lossy());
    let new_secret = format!("new-{}", dir.path().file_name().unwrap().to_string_lossy());
    for aes256 in [false, true] {
        let mut protection = Protection {
            user_password: old_secret.clone(),
            owner_password: format!("{old_secret}-owner"),
            allow_print: false,
            allow_copy: true,
            allow_modify: false,
            allow_annotate: true,
            aes256,
        };
        std::fs::write(
            &old,
            security::protect(&old_raw, None, &protection).unwrap(),
        )
        .unwrap();
        protection.user_password = new_secret.clone();
        protection.owner_password = format!("{new_secret}-owner");
        std::fs::write(
            &new,
            security::protect(&new_raw, None, &protection).unwrap(),
        )
        .unwrap();
        let passwords = OutlinePasswords {
            old: Some(&old_secret),
            new: Some(&new_secret),
        };
        assert!(inspect_revision_outlines(
            &engine,
            &old,
            &new,
            &options(),
            OutlinePasswords::default(),
            &cancel,
            |_| {}
        )
        .is_err());
        let inspection =
            inspect_revision_outlines(&engine, &old, &new, &options(), passwords, &cancel, |_| {})
                .unwrap();
        let candidate = prepare_revision_outlines(
            &engine,
            &old,
            &new,
            &review(&inspection, OutlineClassification::ContentChange),
            passwords,
            &cancel,
            |_| {},
        )
        .unwrap();
        assert!(candidate
            .render_preview(&engine, None, 200, &cancel)
            .is_err());
        let out = dir.path().join(format!("encrypted-{aes256}.pdf"));
        publish_revision_outlines(candidate, &out, &cancel).unwrap();
        assert!(engine.open(&out, None).is_err());
        let reopened = engine.open(&out, Some(&new_secret)).unwrap();
        assert!(!reopened.properties().can_print);
        assert!(!reopened.properties().can_modify);
        assert!(reopened.properties().can_annotate);
        protection.allow_annotate = false;
        std::fs::write(
            &new,
            security::protect(&new_raw, None, &protection).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            inspect_revision_outlines(&engine, &old, &new, &options(), passwords, &cancel, |_| {}),
            Err(EngineError::SecurityRestricted)
        ));
    }
    std::fs::write(&old, &old_raw).unwrap();
    let mut signed = Pdf::load_mem(&new_raw).unwrap();
    signed.add_object(dictionary! {"Type"=>"Sig","Contents"=>Object::string_literal(vec![0;16])});
    signed.save(&new).unwrap();
    assert!(inspect_revision_outlines(
        &engine,
        &old,
        &new,
        &options(),
        OutlinePasswords::default(),
        &cancel,
        |_| {}
    )
    .unwrap_err()
    .to_string()
    .contains("signed"));
}

#[cfg(target_os = "linux")]
#[test]
fn retargeted_source_alias_is_rejected_even_when_the_new_target_has_identical_bytes() {
    use std::os::unix::fs::symlink;
    let engine = engine();
    let dir = tempfile::tempdir().unwrap();
    let (old, new) = files(dir.path(), 0);
    let alias = dir.path().join("selected.pdf");
    let alternate = dir.path().join("alternate.pdf");
    std::fs::copy(&old, &alternate).unwrap();
    symlink(&old, &alias).unwrap();
    let inspection = inspect(&engine, &alias, &new);
    let candidate = prepare(
        &engine,
        &alias,
        &new,
        &review(&inspection, OutlineClassification::ContentChange),
    );
    std::fs::remove_file(&alias).unwrap();
    symlink(&alternate, &alias).unwrap();
    let output = dir.path().join("retargeted.pdf");
    assert!(publish_revision_outlines(candidate, &output, &AtomicBool::new(false)).is_err());
    assert!(!output.exists());
}
