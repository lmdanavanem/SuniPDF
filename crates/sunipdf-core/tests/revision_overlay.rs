use lopdf::{dictionary, Document as Pdf, Object, Stream};
use std::sync::atomic::{AtomicBool, Ordering};
use sunipdf_core::revision_overlay::*;
use sunipdf_core::{security, Engine, EngineError, Protection};

fn fixture(rotation: i64, crop: [i64; 4], content: &str) -> Vec<u8> {
    let mut pdf = Pdf::with_version("1.7");
    let pages = pdf.new_object_id();
    let stream = pdf.add_object(Stream::new(dictionary! {}, content.as_bytes().to_vec()));
    let page = pdf.add_object(dictionary! {
        "Type"=>"Page", "Parent"=>pages, "MediaBox"=>vec![0.into(),0.into(),10000.into(),10000.into()],
        "CropBox"=>crop.into_iter().map(Object::Integer).collect::<Vec<_>>(),
        "Rotate"=>rotation, "Resources"=>dictionary!{}, "Contents"=>stream,
    });
    pdf.objects.insert(
        pages,
        dictionary! {"Type"=>"Pages", "Count"=>1, "Kids"=>vec![page.into()]}.into(),
    );
    let root = pdf.add_object(dictionary! {"Type"=>"Catalog", "Pages"=>pages});
    pdf.trailer.set("Root", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

fn options() -> OverlayOptions {
    OverlayOptions {
        long_side: 128,
        opacity: 1.0,
        ..Default::default()
    }
}

fn pixel_at(image: &sunipdf_core::RenderedPage, size: [f64; 2], point: [f64; 2]) -> &[u8] {
    let x = (point[0] * f64::from(image.width) / size[0]) as u32;
    let y = (point[1] * f64::from(image.height) / size[1]) as u32;
    let i = ((y * image.width + x) * 4) as usize;
    &image.rgba[i..i + 4]
}

#[test]
fn revision_overlay_geometry_copy_permissions_immutable_sources_and_stale_exports() {
    let engine = Engine::new().expect("PDFium is required");
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().join("base.pdf");
    let revision = temp.path().join("revision.pdf");
    let cancel = AtomicBool::new(false);
    for (rotation, size, point) in [
        (0, [200.0, 160.0], [55.0, 110.0]),
        (90, [160.0, 200.0], [50.0, 55.0]),
        (180, [200.0, 160.0], [145.0, 50.0]),
        (270, [160.0, 200.0], [110.0, 145.0]),
    ] {
        let bytes = fixture(rotation, [100, 50, 300, 210], "0 g 140 90 30 20 re f");
        std::fs::write(&base, &bytes).unwrap();
        std::fs::write(&revision, &bytes).unwrap();
        let preview = prepare_overlay(
            &engine,
            &base,
            &revision,
            options(),
            OverlayPasswords::default(),
            &cancel,
        )
        .unwrap();
        let report = preview.report();
        assert_eq!(report["schema"], "sunipdf-revision-overlay");
        assert_eq!(report["base"]["rotation_degrees"], rotation);
        assert_eq!(
            report["base"]["crop_box_user_pt"],
            serde_json::json!([100.0, 50.0, 300.0, 210.0])
        );
        assert_eq!(report["base"]["display_width_pt"], size[0]);
        assert_eq!(report["base"]["display_height_pt"], size[1]);
        assert_eq!(pixel_at(preview.base_image(), size, point), [0, 0, 0, 255]);
        assert_eq!(pixel_at(preview.image(), size, point), [0, 0, 255, 255]);
        let png = preview.export_png(&cancel).unwrap();
        let decoded = image::load_from_memory_with_format(&png, image::ImageFormat::Png).unwrap();
        assert_eq!(
            (decoded.width(), decoded.height()),
            (preview.image().width, preview.image().height)
        );
        assert_eq!(preview.export_json(&cancel).unwrap(), report);
        assert_eq!(std::fs::read(&base).unwrap(), bytes);
        assert_eq!(std::fs::read(&revision).unwrap(), bytes);
    }
    let original = fixture(0, [100, 50, 300, 210], "0 g 140 90 30 20 re f");
    std::fs::write(&base, &original).unwrap();
    std::fs::write(&revision, &original).unwrap();
    let prepared = prepare_overlay(
        &engine,
        &base,
        &revision,
        options(),
        OverlayPasswords::default(),
        &cancel,
    )
    .unwrap();
    std::fs::write(
        &revision,
        fixture(0, [100, 50, 300, 210], "0 g 200 90 30 20 re f"),
    )
    .unwrap();
    assert!(prepared.revalidate_sources(&cancel).is_err());
    assert!(prepared.export_json(&cancel).is_err());
    assert!(prepared.export_png(&cancel).is_err());
    std::fs::write(&revision, &original).unwrap();
    cancel.store(true, Ordering::Relaxed);
    assert!(matches!(
        prepared.export_png(&cancel),
        Err(EngineError::Cancelled)
    ));
    assert!(matches!(
        prepared.export_json(&cancel),
        Err(EngineError::Cancelled)
    ));
    assert!(matches!(
        prepare_overlay(
            &engine,
            &base,
            &revision,
            options(),
            OverlayPasswords::default(),
            &cancel
        ),
        Err(EngineError::Cancelled)
    ));
    cancel.store(false, Ordering::Relaxed);
    let anchor_options = OverlayOptions {
        alignment: OverlayAlignment::TwoPoints {
            base: [[30.0, 40.0], [30.0, 80.0]],
            revision: [[10.0, 10.0], [30.0, 10.0]],
        },
        ..options()
    };
    let anchors = prepare_overlay(
        &engine,
        &base,
        &revision,
        anchor_options.clone(),
        OverlayPasswords::default(),
        &cancel,
    )
    .unwrap();
    assert_eq!(anchors.transform().scale, 2.0);
    assert_eq!(anchors.transform().rotation_degrees, 90.0);
    let manual = prepare_overlay(
        &engine,
        &base,
        &revision,
        OverlayOptions {
            alignment: OverlayAlignment::Manual {
                transform: anchors.transform(),
            },
            ..options()
        },
        OverlayPasswords::default(),
        &cancel,
    )
    .unwrap();
    assert_eq!(manual.image().rgba, anchors.image().rgba);
    let bad_anchors = OverlayOptions {
        alignment: OverlayAlignment::TwoPoints {
            base: [[10.0, 10.0], [201.0, 10.0]],
            revision: [[10.0, 10.0], [30.0, 10.0]],
        },
        ..options()
    };
    assert!(prepare_overlay(
        &engine,
        &base,
        &revision,
        bad_anchors,
        OverlayPasswords::default(),
        &cancel
    )
    .is_err());
    let wide = fixture(0, [0, 0, 9000, 100], "0 g 4000 40 30 20 re f");
    std::fs::write(&revision, &wide).unwrap();
    let wide_preview = prepare_overlay(
        &engine,
        &base,
        &revision,
        options(),
        OverlayPasswords::default(),
        &cancel,
    )
    .unwrap();
    assert_eq!(wide_preview.revision_image().width, 128);
    assert!(wide_preview.revision_image().height <= 2);
    assert_eq!(wide_preview.image().width, wide_preview.base_image().width);
    assert_eq!(
        wide_preview.image().height,
        wide_preview.base_image().height
    );
    assert!(prepare_overlay(
        &engine,
        &base,
        &revision,
        OverlayOptions {
            revision_page: 2,
            ..options()
        },
        OverlayPasswords::default(),
        &cancel
    )
    .is_err());
    let too_large = temp.path().join("too-large.pdf");
    std::fs::File::create(&too_large)
        .unwrap()
        .set_len(sunipdf_core::local_review::MAX_FILE_BYTES + 1)
        .unwrap();
    assert!(prepare_overlay(
        &engine,
        &base,
        &too_large,
        options(),
        OverlayPasswords::default(),
        &cancel
    )
    .is_err());

    let protected_base = temp.path().join("protected-base.pdf");
    let protected_revision = temp.path().join("protected-revision.pdf");
    for aes256 in [false, true] {
        for allow_copy in [false, true] {
            let protect = |user: &str, owner: &str| {
                security::protect(
                    &original,
                    None,
                    &Protection {
                        user_password: user.into(),
                        owner_password: owner.into(),
                        allow_copy,
                        allow_modify: false,
                        allow_annotate: false,
                        aes256,
                        ..Protection::default()
                    },
                )
                .unwrap()
            };
            let a = protect("overlay-base-user", "overlay-base-owner");
            let b = protect("overlay-revision-user", "overlay-revision-owner");
            std::fs::write(&protected_base, &a).unwrap();
            std::fs::write(&protected_revision, &b).unwrap();
            for (base_password, revision_password) in [
                ("overlay-base-user", "overlay-revision-user"),
                ("overlay-base-owner", "overlay-revision-owner"),
            ] {
                let result = prepare_overlay(
                    &engine,
                    &protected_base,
                    &protected_revision,
                    options(),
                    OverlayPasswords {
                        base: Some(base_password),
                        revision: Some(revision_password),
                    },
                    &cancel,
                );
                if allow_copy {
                    assert!(result.unwrap().export_png(&cancel).is_ok());
                } else {
                    assert!(matches!(result, Err(EngineError::SecurityRestricted)));
                    // Check the second source independently of the first source's permission.
                    assert!(matches!(
                        prepare_overlay(
                            &engine,
                            &base,
                            &protected_revision,
                            options(),
                            OverlayPasswords {
                                base: None,
                                revision: Some(revision_password)
                            },
                            &cancel
                        ),
                        Err(EngineError::SecurityRestricted)
                    ));
                }
            }
            assert!(matches!(
                prepare_overlay(
                    &engine,
                    &protected_base,
                    &protected_revision,
                    options(),
                    OverlayPasswords::default(),
                    &cancel
                ),
                Err(EngineError::PasswordRequired)
            ));
            assert_eq!(std::fs::read(&protected_base).unwrap(), a);
            assert_eq!(std::fs::read(&protected_revision).unwrap(), b);
        }
    }
    assert_eq!(std::fs::read(base).unwrap(), original);
    assert_eq!(std::fs::read(revision).unwrap(), wide);
}
