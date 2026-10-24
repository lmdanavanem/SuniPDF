use lopdf::{dictionary, Document as Pdf, Object, Stream};
use std::path::Path;
use std::sync::{Mutex, MutexGuard, OnceLock};
use sunipdf_core::annotation_register::{RegisterFilter, RegisterSort};
use sunipdf_core::plan_zones::*;
use sunipdf_core::quantity_table::QuantityOptions;
use sunipdf_core::{security, Engine, EngineError, Protection};
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

fn zone(id: &str, parent: Option<&str>, bounds: [f32; 4]) -> PlanZone {
    PlanZone {
        id: id.into(),
        name: format!("Room {id}"),
        parent_id: parent.map(str::to_owned),
        bounds,
    }
}
fn zones() -> Vec<PlanZone> {
    vec![
        zone("floor", None, [5., 5., 195., 195.]),
        zone("room", Some("floor"), [20., 20., 90., 100.]),
    ]
}
fn filter(id: &str, descendants: bool) -> RegisterFilter {
    RegisterFilter {
        zone: Some(ZoneFilter {
            page: 1,
            id: id.into(),
            descendants,
        }),
        ..Default::default()
    }
}

#[test]
fn strict_assignment_preserves_hierarchy_and_exposes_boundaries_overlap_and_unknowns() {
    let mut p = PagePlanZones {
        page: 1,
        validity: ZoneValidity::Current,
        zones: zones(),
    };
    let a = assign_zone(&p, Some([30., 30., 40., 40.]));
    assert_eq!(a.status, ZoneAssignmentStatus::Assigned);
    assert_eq!(a.zone_id.as_deref(), Some("room"));
    assert_eq!(
        a.path.iter().map(|z| z.id.as_str()).collect::<Vec<_>>(),
        vec!["floor", "room"]
    );
    assert_eq!(
        assign_zone(&p, Some([80., 30., 110., 40.])).status,
        ZoneAssignmentStatus::Boundary
    );
    assert_eq!(
        assign_zone(&p, Some([20., 30., 40., 40.])).status,
        ZoneAssignmentStatus::Boundary
    );
    assert_eq!(
        assign_zone(&p, Some([196., 196., 199., 199.])).status,
        ZoneAssignmentStatus::Unassigned
    );
    assert_eq!(
        assign_zone(&p, None).status,
        ZoneAssignmentStatus::GeometryUnknown
    );
    p.zones.push(zone("overlap", None, [25., 25., 85., 85.]));
    assert_eq!(
        assign_zone(&p, Some([30., 30., 40., 40.])).status,
        ZoneAssignmentStatus::Overlap
    );
    p.validity = ZoneValidity::Stale;
    assert_eq!(
        assign_zone(&p, Some([30., 30., 40., 40.])).status,
        ZoneAssignmentStatus::Stale
    );
}
#[test]
fn imports_bound_identifiers_hierarchy_cycles_unknown_fields_and_size() {
    assert_eq!(
        parse_zones(&encode_zones(&zones()).unwrap()).unwrap(),
        zones()
    );
    let mut invalid = zones();
    invalid[1].parent_id = Some("missing".into());
    assert!(encode_zones(&invalid).is_err());
    invalid = zones();
    invalid[0].parent_id = Some("room".into());
    assert!(encode_zones(&invalid).is_err());
    invalid = zones();
    invalid[1].id = "floor".into();
    assert!(encode_zones(&invalid).is_err());
    invalid = zones();
    invalid[1].bounds = [0., 0., 100., 100.];
    assert!(encode_zones(&invalid).is_err());
    invalid = (0..65)
        .map(|i| zone(&format!("z{i}"), None, [1., 1., 100., 100.]))
        .collect();
    assert!(encode_zones(&invalid).is_err());
    invalid = (0..9)
        .map(|i| {
            zone(
                &format!("z{i}"),
                if i == 0 { None } else { Some("unused") },
                [
                    i as f32 + 1.,
                    i as f32 + 1.,
                    100. - i as f32,
                    100. - i as f32,
                ],
            )
        })
        .collect();
    for (i, z) in invalid.iter_mut().enumerate() {
        z.parent_id = (i > 0).then(|| format!("z{}", i - 1));
    }
    assert!(encode_zones(&invalid).is_err());
    assert!(parse_zones(br#"{"version":99,"zones":[]}"#).is_err());
    assert!(parse_zones(br#"{"version":1,"zones":[],"script":"ignored"}"#).is_err());
    assert!(parse_zones(&vec![b' '; MAX_RECORD_BYTES + 1]).is_err());
}
#[test]
fn four_rotations_preserve_content_annotations_hierarchy_and_exact_quantity_selection() {
    let engine = engine();
    for rotation in [0, 90, 180, 270] {
        let original = fixture(rotation, "0 g 110 60 20 20 re f");
        let mut doc = engine
            .open_bytes(original.clone(), None, Path::new("zones.pdf"))
            .unwrap();
        doc.add_count_marker(0, (40., 40.), 8., "Lights", None)
            .unwrap();
        doc.add_count_marker(0, (140., 140.), 8., "Lights", None)
            .unwrap();
        let before = doc.snapshot().unwrap();
        let prepared = doc.prepare_plan_zones(0, &zones(), false).unwrap();
        assert_eq!(prepared.report().affected_rows(), 2);
        let image = doc.render_plan_zone_preview(&prepared, 200).unwrap();
        assert_eq!((image.width, image.height), (200, 200));
        assert_eq!(
            doc.plan_zones(0).unwrap().zones.len(),
            0,
            "preview must not edit"
        );
        doc.apply_plan_zones(prepared).unwrap();
        let saved = doc.snapshot().unwrap();
        let reopened = engine
            .open_bytes(saved.clone(), None, Path::new("reopened.pdf"))
            .unwrap();
        assert_eq!(reopened.plan_zones(0).unwrap().zones, zones());
        let register = reopened.annotation_register().unwrap();
        assert_eq!(
            register
                .select(&filter("floor", true), RegisterSort::Page, false)
                .len(),
            2
        );
        assert_eq!(
            register
                .select(&filter("floor", false), RegisterSort::Page, false)
                .len(),
            1
        );
        assert_eq!(
            register
                .select(&filter("room", true), RegisterSort::Page, false)
                .len(),
            1
        );
        let table = reopened.quantity_table().unwrap();
        let options = QuantityOptions {
            filter: filter("floor", true),
            ..Default::default()
        };
        let summary = table.summary(&options).unwrap();
        assert_eq!(summary.matched_rows, 2);
        assert_eq!(summary.quantity_rows, 2);
        let csv = register
            .csv(&filter("room", true), RegisterSort::Page, false)
            .unwrap();
        assert!(csv.contains("zone_status"));
        assert_eq!(csv.lines().count(), 2);
        let old = Pdf::load_mem(&before).unwrap();
        let new = Pdf::load_mem(&saved).unwrap();
        assert_eq!(
            old.get_page_content(old.get_pages()[&1]),
            new.get_page_content(new.get_pages()[&1])
        );
    }
}
#[test]
fn moving_annotations_reassigns_without_rewriting_zones_and_stale_candidate_cannot_commit() {
    let engine = engine();
    let mut doc = engine
        .open_bytes(fixture(0, "q Q"), None, Path::new("move.pdf"))
        .unwrap();
    let index = doc
        .add_count_marker(0, (40., 40.), 8., "Lights", None)
        .unwrap();
    let candidate = doc.prepare_plan_zones(0, &zones(), false).unwrap();
    doc.apply_plan_zones(candidate).unwrap();
    let candidate = doc.prepare_plan_zones(0, &[], false).unwrap();
    doc.move_annotation(0, index, 100., 100.).unwrap();
    assert!(doc.apply_plan_zones(candidate).is_err());
    assert_eq!(doc.plan_zones(0).unwrap().zones, zones());
    assert_eq!(
        doc.annotation_register().unwrap().rows[0]
            .zone
            .zone_id
            .as_deref(),
        Some("floor")
    );
}
fn rewrite(bytes: &[u8], change: impl FnOnce(&mut Pdf, lopdf::ObjectId)) -> Vec<u8> {
    let mut p = Pdf::load_mem(bytes).unwrap();
    let page = p.get_pages()[&1];
    change(&mut p, page);
    let mut b = Vec::new();
    p.save_to(&mut b).unwrap();
    b
}
#[test]
fn changed_content_rotation_and_rendering_context_are_historical_until_explicit_replacement() {
    let engine = engine();
    let mut doc = engine
        .open_bytes(fixture(0, "q Q"), None, Path::new("stale.pdf"))
        .unwrap();
    doc.add_count_marker(0, (40., 40.), 8., "Lights", None)
        .unwrap();
    let prepared = doc.prepare_plan_zones(0, &zones(), false).unwrap();
    doc.apply_plan_zones(prepared).unwrap();
    let saved = doc.snapshot().unwrap();
    for mutation in 0..3 {
        let changed = rewrite(&saved, |pdf, page| {
            if mutation == 0 {
                let stream = pdf.add_object(Stream::new(
                    dictionary! {},
                    b"0 g 130 80 10 10 re f".to_vec(),
                ));
                pdf.get_dictionary_mut(page)
                    .unwrap()
                    .set("Contents", stream);
            } else if mutation == 1 {
                pdf.get_dictionary_mut(page).unwrap().set("Rotate", 90);
            } else {
                pdf.get_dictionary_mut(page)
                    .unwrap()
                    .set("Group", dictionary! {"S"=>"Transparency","CS"=>"DeviceRGB"});
            }
        });
        let mut changed = engine
            .open_bytes(changed, None, Path::new("changed.pdf"))
            .unwrap();
        assert_eq!(changed.plan_zones(0).unwrap().validity, ZoneValidity::Stale);
        let register = changed.annotation_register().unwrap();
        assert_eq!(register.rows[0].zone.status, ZoneAssignmentStatus::Stale);
        assert!(register
            .validate_zone_filter(&filter("room", true))
            .is_err());
        assert!(register
            .csv(&filter("room", true), RegisterSort::Page, false)
            .is_err());
        assert!(changed
            .quantity_table()
            .unwrap()
            .summary(&QuantityOptions {
                filter: filter("room", true),
                ..Default::default()
            })
            .is_err());
        assert!(changed.prepare_plan_zones(0, &zones(), false).is_err());
        let p = changed.prepare_plan_zones(0, &zones(), true).unwrap();
        changed.apply_plan_zones(p).unwrap();
        assert_eq!(
            changed.plan_zones(0).unwrap().validity,
            ZoneValidity::Current
        );
    }
}
#[test]
fn empty_existing_zone_is_distinct_from_unknown_zone_and_unresolved_geometry_blocks_totals() {
    let engine = engine();
    let mut doc = engine
        .open_bytes(fixture(0, "q Q"), None, Path::new("coverage.pdf"))
        .unwrap();
    let prepared = doc.prepare_plan_zones(0, &zones(), false).unwrap();
    doc.apply_plan_zones(prepared).unwrap();
    let register = doc.annotation_register().unwrap();
    assert!(register.validate_zone_filter(&filter("room", true)).is_ok());
    let mut conflicting_page = filter("room", true);
    conflicting_page.page = Some(2);
    assert!(register.validate_zone_filter(&conflicting_page).is_err());
    assert!(register
        .validate_zone_filter(&filter("missing", true))
        .is_err());
    assert!(
        !register.json(&filter("missing", true), RegisterSort::Page, false)["complete"]
            .as_bool()
            .unwrap()
    );
    let invalid = rewrite(&doc.snapshot().unwrap(), |pdf, page| {
        let a = pdf.add_object(dictionary! {"Type"=>"Annot","Subtype"=>"Stamp"});
        pdf.get_dictionary_mut(page)
            .unwrap()
            .set("Annots", vec![Object::Reference(a)]);
    });
    let invalid = engine
        .open_bytes(invalid, None, Path::new("unknown.pdf"))
        .unwrap();
    let register = invalid.annotation_register().unwrap();
    assert_eq!(
        register.rows[0].zone.status,
        ZoneAssignmentStatus::GeometryUnknown
    );
    assert!(register
        .validate_zone_filter(&filter("room", true))
        .is_err());
    assert_eq!(
        register.zone_coverage(&RegisterFilter::default()).counts["geometry_unknown"],
        1
    );
}
#[test]
fn restricted_and_signed_documents_reject_edits_and_owner_output_keeps_encryption() {
    let engine = engine();
    let raw = fixture(0, "q Q");
    let owner = format!("fixture-owner-{}", std::process::id());
    let user = format!("fixture-user-{}", std::process::id());
    let encrypted = security::protect(
        &raw,
        None,
        &Protection {
            user_password: user.clone(),
            owner_password: owner.clone(),
            allow_print: true,
            allow_copy: false,
            allow_modify: false,
            allow_annotate: true,
            aes256: true,
        },
    )
    .unwrap();
    let user_doc = engine
        .open_bytes(encrypted.clone(), Some(&user), Path::new("protected.pdf"))
        .unwrap();
    assert!(matches!(
        user_doc.prepare_plan_zones(0, &zones(), false),
        Err(EngineError::SecurityRestricted)
    ));
    let owner_doc = engine
        .open_bytes(encrypted.clone(), Some(&owner), Path::new("protected.pdf"))
        .unwrap();
    // Loading with the owner credential does not waive the declared /P permissions.
    assert!(!owner_doc.properties().can_copy && !owner_doc.properties().can_modify);
    assert!(matches!(
        owner_doc.prepare_plan_zones(0, &zones(), false),
        Err(EngineError::SecurityRestricted)
    ));
    assert!(owner_doc.plan_zones(0).unwrap().zones.is_empty());
    let permitted = security::protect(
        &raw,
        None,
        &Protection {
            user_password: user.clone(),
            owner_password: owner.clone(),
            allow_copy: true,
            allow_modify: true,
            aes256: true,
            ..Default::default()
        },
    )
    .unwrap();
    let mut owner_doc = engine
        .open_bytes(permitted, Some(&owner), Path::new("permitted.pdf"))
        .unwrap();
    assert!(owner_doc.properties().can_copy && owner_doc.properties().can_modify);
    let p = owner_doc.prepare_plan_zones(0, &zones(), false).unwrap();
    owner_doc.apply_plan_zones(p).unwrap();
    let saved = owner_doc.snapshot().unwrap();
    let check = Pdf::load_mem(&saved).unwrap();
    assert!(check.is_encrypted());
    assert!(engine
        .open_bytes(saved.clone(), None, Path::new("saved.pdf"))
        .is_err());
    for password in [&user, &owner] {
        let reopened = engine
            .open_bytes(saved.clone(), Some(password), Path::new("saved.pdf"))
            .unwrap();
        assert_eq!(reopened.plan_zones(0).unwrap().zones, zones());
        let properties = reopened.properties();
        assert!(properties.encrypted && properties.can_copy && properties.can_modify);
    }
    let signed = rewrite(&raw, |pdf, _| {
        let sig=pdf.add_object(dictionary!{"FT"=>"Sig","T"=>Object::string_literal("Test signature"),"V"=>dictionary!{"Type"=>"Sig","ByteRange"=>vec![0.into(),1.into(),2.into(),3.into()],"Contents"=>Object::string_literal("test")}});
        let root = pdf.trailer.get(b"Root").unwrap().as_reference().unwrap();
        pdf.get_dictionary_mut(root).unwrap().set(
            "AcroForm",
            dictionary! {"Fields"=>vec![Object::Reference(sig)]},
        );
    });
    let signed = engine
        .open_bytes(signed, None, Path::new("signed.pdf"))
        .unwrap();
    assert!(signed.signature_count() > 0);
    assert!(signed.prepare_plan_zones(0, &zones(), false).is_err());
}
