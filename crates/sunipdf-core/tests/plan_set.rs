use lopdf::{dictionary, Document as Pdf, Object, Stream};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex, MutexGuard, OnceLock,
};
use sunipdf_core::{plan_set::*, security, Engine, EngineError, Protection};

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

fn pdf(pages: usize, text: &str) -> Vec<u8> {
    let mut document = Pdf::with_version("1.7");
    let root_pages = document.new_object_id();
    let font = document.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica"
    });
    let mut kids = Vec::new();
    for index in 0..pages {
        let content = document.add_object(Stream::new(
            dictionary! {},
            format!("BT /F1 12 Tf 20 80 Td ({text} {index}) Tj ET").into_bytes(),
        ));
        let page = document.add_object(dictionary! {
            "Type" => "Page", "Parent" => root_pages,
            "MediaBox" => vec![0.into(), 0.into(), 200.into(), 160.into()],
            "Resources" => dictionary! {"Font" => dictionary! {"F1" => font}},
            "Contents" => content,
        });
        kids.push(Object::Reference(page));
    }
    document.objects.insert(
        root_pages,
        dictionary! {
            "Type" => "Pages", "Count" => pages as i64, "Kids" => kids,
        }
        .into(),
    );
    let root = document.add_object(dictionary! {"Type" => "Catalog", "Pages" => root_pages});
    document.trailer.set("Root", root);
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).unwrap();
    bytes
}

fn basic_set() -> PlanSet {
    let mut set = PlanSet::new("Fixture set").unwrap();
    set.files.push(PlanFile {
        id: "file-a".into(),
        path: "plans/a.pdf".into(),
        sha256: "ab".repeat(32),
        byte_len: 100,
        page_count: 2,
    });
    set.sheets.push(PlanSheet {
        id: "sheet-a".into(),
        file_id: "file-a".into(),
        page: 1,
        number: "A-01".into(),
        title: "Ground floor".into(),
        discipline: "Architecture".into(),
        revision: "01".into(),
    });
    set
}

#[test]
fn versioned_manifest_roundtrips_and_rejects_ambiguous_or_unbounded_data() {
    let set = basic_set();
    assert_eq!(parse(&encode(&set).unwrap()).unwrap(), set);
    assert!(PlanSet::new("  ").is_err());
    for modify in [
        |s: &mut PlanSet| s.version = 2,
        |s: &mut PlanSet| s.id.clear(),
        |s: &mut PlanSet| s.files[0].sha256 = "z".repeat(64),
        |s: &mut PlanSet| s.files[0].byte_len = MAX_FILE_BYTES + 1,
        |s: &mut PlanSet| s.files[0].page_count = MAX_PAGES + 1,
        |s: &mut PlanSet| s.sheets[0].file_id = "missing".into(),
        |s: &mut PlanSet| s.sheets[0].page = 0,
        |s: &mut PlanSet| s.sheets[0].number = "\nunsafe".into(),
        |s: &mut PlanSet| s.files.push(s.files[0].clone()),
        |s: &mut PlanSet| s.sheets.push(s.sheets[0].clone()),
    ] {
        let mut invalid = set.clone();
        modify(&mut invalid);
        assert!(encode(&invalid).is_err());
    }
    let bytes = encode(&set).unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    json["password"] = "must-not-persist".into();
    assert!(parse(&serde_json::to_vec(&json).unwrap()).is_err());
    assert!(parse(&vec![b' '; MAX_MANIFEST_BYTES + 1]).is_err());
    let duplicate_key = String::from_utf8(bytes).unwrap().replacen(
        "\"version\": 1",
        "\"version\": 1, \"version\": 1",
        1,
    );
    assert!(parse(duplicate_key.as_bytes()).is_err());
    let mut too_many = set.clone();
    too_many.files = (0..=MAX_FILES)
        .map(|i| PlanFile {
            id: format!("file-{i}"),
            ..set.files[0].clone()
        })
        .collect();
    assert!(encode(&too_many).is_err());
}

#[test]
fn manifest_rebase_preserves_missing_references_and_never_overwrites_sources() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    std::fs::create_dir(&project).unwrap();
    let original = project.join("set.json");
    let set = basic_set();
    save_new(&set, &original, &original).unwrap();
    assert_eq!(load(&original).unwrap(), set);
    let outer = temp.path().join("copy.json");
    save_new(&set, &original, &outer).unwrap();
    assert_eq!(load(&outer).unwrap().files[0].path, "project/plans/a.pdf");
    assert_eq!(load(&original).unwrap(), set);
    let before = std::fs::read(&outer).unwrap();
    assert!(matches!(
        save_new(&set, &original, &outer),
        Err(EngineError::AlreadyExists(_))
    ));
    assert_eq!(std::fs::read(&outer).unwrap(), before);
    let other = temp.path().join("other");
    std::fs::create_dir(&other).unwrap();
    assert!(save_new(&set, &original, &other.join("escape.json")).is_err());
    assert!(!other.join("escape.json").exists());
    let mut collision = set.clone();
    collision.files[0].path = "absent.pdf".into();
    assert!(save_new(&collision, &original, &project.join("absent.pdf")).is_err());
    assert!(!project.join("absent.pdf").exists());
}

#[test]
fn real_sources_relocate_and_changed_revisions_require_explicit_atomic_refresh() {
    let engine = engine();
    let temp = tempfile::tempdir().unwrap();
    let original = temp.path().join("original");
    let moved = temp.path().join("moved");
    for root in [&original, &moved] {
        std::fs::create_dir_all(root.join("Plans été")).unwrap();
    }
    let manifest = original.join("set.json");
    let source = original.join("Plans été/A-01.pdf");
    let second = original.join("M-01.pdf");
    let bytes = pdf(2, "Original");
    std::fs::write(&source, &bytes).unwrap();
    std::fs::write(&second, pdf(1, "Mechanical")).unwrap();
    let mut set = PlanSet::new("Project").unwrap();
    let ids = add_pdf(
        &mut set,
        &engine,
        &manifest,
        &source,
        None,
        &AddOptions {
            discipline: "A".into(),
            revision: "01".into(),
        },
    )
    .unwrap();
    assert_eq!(ids.len(), 2);
    add_pdf(
        &mut set,
        &engine,
        &manifest,
        &second,
        None,
        &AddOptions::default(),
    )
    .unwrap();
    let cancel = AtomicBool::new(false);
    let report = inspect(&engine, &manifest, &set, None, &cancel, |_, _| {}).unwrap();
    assert!(report.complete(), "{}", report.json());
    let navigation = read_sheet(&manifest, &set, &ids[1]).unwrap();
    assert_eq!(navigation.page, 2);
    assert_eq!(navigation.bytes, bytes);
    assert_eq!(navigation.sha256, set.files[0].sha256);
    save_new(&set, &manifest, &manifest).unwrap();
    std::fs::copy(&source, moved.join("Plans été/A-01.pdf")).unwrap();
    std::fs::copy(&second, moved.join("M-01.pdf")).unwrap();
    std::fs::copy(&manifest, moved.join("set.json")).unwrap();
    let moved_manifest = moved.join("set.json");
    let moved_set = load(&moved_manifest).unwrap();
    assert!(inspect(
        &engine,
        &moved_manifest,
        &moved_set,
        None,
        &cancel,
        |_, _| {}
    )
    .unwrap()
    .complete());
    assert_eq!(
        read_sheet(&moved_manifest, &moved_set, &ids[0])
            .unwrap()
            .bytes,
        bytes
    );
    std::fs::write(&source, pdf(1, "Revision")).unwrap();
    let before = set.clone();
    let report = inspect(&engine, &manifest, &set, None, &cancel, |_, _| {}).unwrap();
    assert_eq!(report.files[0].status, ReferenceStatus::Changed);
    assert_eq!(set, before);
    assert!(read_sheet(&manifest, &set, &ids[0]).is_err());
    let file_id = set.files[0].id.clone();
    refresh_exact(&mut set, &engine, &manifest, &file_id, None, None).unwrap();
    assert_ne!(set.files[0].sha256, before.files[0].sha256);
    assert_eq!(set.sheets, before.sheets);
    let report = inspect(&engine, &manifest, &set, None, &cancel, |_, _| {}).unwrap();
    assert_eq!(report.sheets[0].status, ReferenceStatus::Ready);
    assert_eq!(report.sheets[1].status, ReferenceStatus::PageOutOfRange);
    assert!(!report.complete());
    let broken = original.join("broken.pdf");
    std::fs::write(&broken, b"invalid PDF").unwrap();
    let before = set.clone();
    assert!(refresh_exact(&mut set, &engine, &manifest, &file_id, Some(&broken), None).is_err());
    assert_eq!(set, before);
    assert!(add_pdf(
        &mut set,
        &engine,
        &manifest,
        &second,
        None,
        &AddOptions::default()
    )
    .is_err());
    assert_eq!(set, before);
    assert_eq!(
        std::fs::read(moved.join("Plans été/A-01.pdf")).unwrap(),
        bytes
    );
}

#[test]
fn statuses_cover_missing_unsafe_duplicate_cancelled_and_protected_sources() {
    let engine = engine();
    let temp = tempfile::tempdir().unwrap();
    let manifest = temp.path().join("set.json");
    let source = temp.path().join("source.pdf");
    let bytes = pdf(1, "Fixture");
    std::fs::write(&source, &bytes).unwrap();
    let mut set = PlanSet::new("Inspection").unwrap();
    add_pdf(
        &mut set,
        &engine,
        &manifest,
        &source,
        None,
        &AddOptions::default(),
    )
    .unwrap();
    let cancel = AtomicBool::new(false);
    let inspect_set =
        |set: &PlanSet| inspect(&engine, &manifest, set, None, &cancel, |_, _| {}).unwrap();
    for path in [
        "../source.pdf",
        "/source.pdf",
        "C:/source.pdf",
        "https://example.test/a.pdf",
        "a\\b.pdf",
        "dir//x.pdf",
        "dir/../x.pdf",
        "NUL.pdf",
        "a.pdf:stream",
        "name./x.pdf",
    ] {
        let mut unsafe_set = set.clone();
        unsafe_set.files[0].path = path.into();
        assert_eq!(
            inspect_set(&unsafe_set).files[0].status,
            ReferenceStatus::UnsafePath,
            "{path}"
        );
        assert!(read_sheet(&manifest, &unsafe_set, &set.sheets[0].id).is_err());
        assert!(save_new(&unsafe_set, &manifest, &temp.path().join("unsafe.json")).is_err());
    }
    let mut missing = set.clone();
    missing.files[0].path = "missing.pdf".into();
    assert_eq!(
        inspect_set(&missing).files[0].status,
        ReferenceStatus::Missing
    );
    let mut duplicate = set.clone();
    duplicate.sheets.push(PlanSheet {
        id: "duplicate".into(),
        number: "Other number".into(),
        ..set.sheets[0].clone()
    });
    assert!(inspect_set(&duplicate)
        .sheets
        .iter()
        .all(|row| row.status == ReferenceStatus::Duplicate));
    assert!(read_sheet(&manifest, &duplicate, &set.sheets[0].id).is_err());
    let mut duplicate_file = set.clone();
    duplicate_file.files.push(PlanFile {
        id: "another-file".into(),
        ..set.files[0].clone()
    });
    assert!(inspect_set(&duplicate_file)
        .files
        .iter()
        .all(|row| row.status == ReferenceStatus::Duplicate));
    assert!(read_sheet(&manifest, &duplicate_file, &set.sheets[0].id).is_err());
    cancel.store(true, Ordering::Relaxed);
    let stopped = inspect_set(&set);
    assert!(stopped.cancelled && !stopped.complete());
    assert_eq!(stopped.files[0].status, ReferenceStatus::Cancelled);
    cancel.store(false, Ordering::Relaxed);
    for aes256 in [false, true] {
        let protected = temp.path().join(format!("protected-{aes256}.pdf"));
        let encrypted = security::protect(
            &bytes,
            None,
            &Protection {
                user_password: "plan-fixture-user".into(),
                owner_password: "plan-fixture-owner".into(),
                allow_copy: false,
                aes256,
                ..Protection::default()
            },
        )
        .unwrap();
        std::fs::write(&protected, &encrypted).unwrap();
        let mut protected_set = PlanSet::new("Protected sheets").unwrap();
        assert!(matches!(
            add_pdf(
                &mut protected_set,
                &engine,
                &manifest,
                &protected,
                None,
                &AddOptions::default()
            ),
            Err(EngineError::PasswordRequired)
        ));
        assert!(protected_set.files.is_empty() && protected_set.sheets.is_empty());
        add_pdf(
            &mut protected_set,
            &engine,
            &manifest,
            &protected,
            Some("plan-fixture-user"),
            &AddOptions::default(),
        )
        .unwrap();
        assert_eq!(
            inspect_set(&protected_set).files[0].status,
            ReferenceStatus::PasswordRequired
        );
        assert!(inspect(
            &engine,
            &manifest,
            &protected_set,
            Some("plan-fixture-user"),
            &cancel,
            |_, _| {}
        )
        .unwrap()
        .complete());
        let serialized = String::from_utf8(encode(&protected_set).unwrap()).unwrap();
        assert!(
            !serialized.contains("plan-fixture-user") && !serialized.contains("plan-fixture-owner")
        );
        assert_eq!(std::fs::read(&protected).unwrap(), encrypted);
    }
    assert_eq!(std::fs::read(&source).unwrap(), bytes);
}

#[test]
fn relink_is_explicit_and_outside_sources_or_oversized_inputs_leave_set_unchanged() {
    let engine = engine();
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    std::fs::create_dir(&root).unwrap();
    let manifest = root.join("set.json");
    let source = root.join("source.pdf");
    let moved = root.join("moved.pdf");
    let outside = temp.path().join("outside.pdf");
    let bytes = pdf(1, "Project");
    std::fs::write(&source, &bytes).unwrap();
    std::fs::write(&outside, &bytes).unwrap();
    let mut set = PlanSet::new("Project").unwrap();
    add_pdf(
        &mut set,
        &engine,
        &manifest,
        &source,
        None,
        &AddOptions::default(),
    )
    .unwrap();
    let original = set.clone();
    #[cfg(windows)]
    {
        let alias = std::path::PathBuf::from(source.to_string_lossy().to_uppercase());
        let mut alias_set = PlanSet::new("Case alias").unwrap();
        add_pdf(
            &mut alias_set,
            &engine,
            &manifest,
            &alias,
            None,
            &AddOptions::default(),
        )
        .unwrap();
        assert_eq!(alias_set.files[0].sha256, original.files[0].sha256);
        assert!(inspect(
            &engine,
            &manifest,
            &alias_set,
            None,
            &AtomicBool::new(false),
            |_, _| {}
        )
        .unwrap()
        .complete());
    }
    assert!(add_pdf(
        &mut set,
        &engine,
        &manifest,
        &outside,
        None,
        &AddOptions::default()
    )
    .is_err());
    assert_eq!(set, original);
    let large = root.join("large.pdf");
    std::fs::File::create(&large)
        .unwrap()
        .set_len(MAX_FILE_BYTES + 1)
        .unwrap();
    assert!(add_pdf(
        &mut set,
        &engine,
        &manifest,
        &large,
        None,
        &AddOptions::default()
    )
    .is_err());
    assert_eq!(set, original);
    std::fs::rename(&source, &moved).unwrap();
    let file_id = set.files[0].id.clone();
    assert!(refresh_exact(&mut set, &engine, &manifest, &file_id, None, None).is_err());
    assert_eq!(set, original);
    refresh_exact(&mut set, &engine, &manifest, &file_id, Some(&moved), None).unwrap();
    assert_eq!(set.files[0].sha256, original.files[0].sha256);
    assert_eq!(set.files[0].path, "moved.pdf");
    assert_eq!(set.sheets, original.sheets);
    let cancel = AtomicBool::new(false);
    assert!(inspect(&engine, &manifest, &set, None, &cancel, |_, _| {})
        .unwrap()
        .complete());
    assert_eq!(std::fs::read(&outside).unwrap(), bytes);
}

#[cfg(unix)]
#[test]
fn symbolic_links_cannot_escape_the_manifest_directory() {
    use std::os::unix::fs::symlink;
    let engine = engine();
    let temp = tempfile::tempdir().unwrap();
    let inside = temp.path().join("inside");
    let outside = temp.path().join("outside");
    std::fs::create_dir(&inside).unwrap();
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("a.pdf"), pdf(1, "Outside")).unwrap();
    symlink(&outside, inside.join("link")).unwrap();
    let mut set = basic_set();
    set.files[0].path = "link/a.pdf".into();
    let manifest = inside.join("set.json");
    let report = inspect(
        &engine,
        &manifest,
        &set,
        None,
        &AtomicBool::new(false),
        |_, _| {},
    )
    .unwrap();
    assert_eq!(report.files[0].status, ReferenceStatus::UnsafePath);
    assert!(save_new(&set, &manifest, &inside.join("copy.json")).is_err());
}

#[cfg(windows)]
#[test]
fn directory_junction_cannot_escape_the_manifest_directory() {
    use std::os::windows::process::CommandExt;
    let engine = engine();
    let temp = tempfile::tempdir().unwrap();
    let inside = temp.path().join("inside");
    let outside = temp.path().join("outside");
    std::fs::create_dir(&inside).unwrap();
    std::fs::create_dir(&outside).unwrap();
    let bytes = pdf(1, "Outside");
    std::fs::write(outside.join("a.pdf"), &bytes).unwrap();
    // Junction creation is unprivileged and both targets belong to this test directory.
    let created = std::process::Command::new("cmd.exe")
        .creation_flags(0x0800_0000)
        .args(["/D", "/C", "mklink", "/J"])
        .arg(inside.join("link"))
        .arg(&outside)
        .output()
        .unwrap();
    assert!(
        created.status.success(),
        "junction fixture: {}",
        String::from_utf8_lossy(&created.stderr)
    );
    let mut set = basic_set();
    set.files[0].path = "link/a.pdf".into();
    let manifest = inside.join("set.json");
    let report = inspect(
        &engine,
        &manifest,
        &set,
        None,
        &AtomicBool::new(false),
        |_, _| {},
    )
    .unwrap();
    assert_eq!(report.files[0].status, ReferenceStatus::UnsafePath);
    assert!(save_new(&set, &manifest, &inside.join("copy.json")).is_err());
    assert!(read_sheet(&manifest, &set, &set.sheets[0].id).is_err());
    assert!(!inside.join("copy.json").exists());
    assert_eq!(std::fs::read(outside.join("a.pdf")).unwrap(), bytes);
}
