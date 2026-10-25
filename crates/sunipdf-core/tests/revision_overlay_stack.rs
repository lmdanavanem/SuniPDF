use lopdf::{dictionary, Document as Pdf, Object, Stream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use sunipdf_core::revision_overlay::stack::*;
use sunipdf_core::revision_overlay::{
    prepare_overlay, OverlayAlignment, OverlayOptions, OverlayPasswords,
};
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

fn options(paths: &[PathBuf]) -> OverlayStackOptions {
    OverlayStackOptions {
        layers: paths
            .iter()
            .enumerate()
            .map(|(index, path)| OverlayStackLayer::new(path.clone(), 1, index))
            .collect(),
        long_side: 128,
    }
}

#[test]
fn stack_four_layers_crop_rotations_png_and_report_preserve_exact_sources() {
    let engine = Engine::new().expect("PDFium is required");
    let temp = tempfile::tempdir().unwrap();
    let paths: Vec<_> = (0..4)
        .map(|index| temp.path().join(format!("layer-{index}.pdf")))
        .collect();
    let cancel = AtomicBool::new(false);
    for rotation in [0, 90, 180, 270] {
        let bytes = fixture(rotation, [100, 50, 300, 210], "0 g 140 90 30 20 re f");
        for path in &paths {
            std::fs::write(path, &bytes).unwrap();
        }
        let preview = prepare_overlay_stack(&engine, options(&paths), &[None; 4], &cancel).unwrap();
        let report = preview.report();
        assert_eq!(report["schema"], "sunipdf-revision-overlay-stack");
        assert_eq!(report["version"], 1);
        assert_eq!(report["layers"].as_array().unwrap().len(), 4);
        for index in 0..4 {
            assert_eq!(report["layers"][index]["index"], index);
            assert_eq!(
                report["layers"][index]["source"]["rotation_degrees"],
                rotation
            );
            assert_eq!(
                report["layers"][index]["source"]["crop_box_user_pt"],
                serde_json::json!([100.0, 50.0, 300.0, 210.0])
            );
            assert!(preview.options().layers[index].path.is_absolute());
            assert_eq!(std::fs::read(&paths[index]).unwrap(), bytes);
        }
        assert!(report["retained_raster_bytes"].as_u64().unwrap() <= MAX_RASTER_BYTES as u64);
        assert!(preview.layer_image(4).is_none());
        assert!(preview.transform(4).is_none());
        let png = preview.export_png(&cancel).unwrap();
        let decoded = image::load_from_memory_with_format(&png, image::ImageFormat::Png)
            .unwrap()
            .to_rgba8();
        assert_eq!(decoded.as_raw(), &preview.image().rgba);
        assert_eq!(preview.export_json(&cancel).unwrap(), report);
        let legacy = prepare_overlay(
            &engine,
            &paths[0],
            &paths[1],
            OverlayOptions {
                long_side: 128,
                ..Default::default()
            },
            OverlayPasswords::default(),
            &cancel,
        )
        .unwrap();
        let pair =
            prepare_overlay_stack(&engine, options(&paths[..2]), &[None; 2], &cancel).unwrap();
        assert_eq!(legacy.image().rgba, pair.image().rgba);
        assert_eq!(legacy.report()["version"], 1);
        assert_eq!(legacy.report()["schema"], "sunipdf-revision-overlay");
    }
    let mut aligned_options = options(&paths);
    aligned_options.layers[1].alignment = OverlayAlignment::TwoPoints {
        base: [[30.0, 40.0], [30.0, 80.0]],
        revision: [[10.0, 10.0], [30.0, 10.0]],
    };
    let aligned =
        prepare_overlay_stack(&engine, aligned_options.clone(), &[None; 4], &cancel).unwrap();
    assert_eq!(aligned.transform(1).unwrap().scale, 2.0);
    assert_eq!(aligned.transform(1).unwrap().rotation_degrees, 90.0);
    aligned_options.layers[1].alignment = OverlayAlignment::Manual {
        transform: aligned.transform(1).unwrap(),
    };
    let manual = prepare_overlay_stack(&engine, aligned_options, &[None; 4], &cancel).unwrap();
    assert_eq!(aligned.image().rgba, manual.image().rgba);
}

#[test]
fn stack_refuses_hidden_stale_missing_oversized_sources_bad_pages_and_cancellation() {
    let engine = Engine::new().expect("PDFium is required");
    let temp = tempfile::tempdir().unwrap();
    let paths: Vec<_> = (0..3)
        .map(|index| temp.path().join(format!("layer-{index}.pdf")))
        .collect();
    let bytes = fixture(0, [0, 0, 200, 160], "0 g 40 40 30 20 re f");
    for path in &paths {
        std::fs::write(path, &bytes).unwrap();
    }
    let cancel = AtomicBool::new(false);
    let mut opts = options(&paths);
    opts.layers[2].visible = false;
    let preview = prepare_overlay_stack(&engine, opts.clone(), &[None; 3], &cancel).unwrap();
    std::fs::write(
        &paths[2],
        fixture(0, [0, 0, 200, 160], "0 g 90 40 30 20 re f"),
    )
    .unwrap();
    assert!(preview.revalidate_sources(&cancel).is_err());
    assert!(preview.export_json(&cancel).is_err());
    assert!(preview.export_png(&cancel).is_err());
    std::fs::write(&paths[2], &bytes).unwrap();
    preview.revalidate_sources(&cancel).unwrap();
    assert!(prepare_overlay_stack(&engine, opts.clone(), &[None; 2], &cancel).is_err());
    let mut bad = opts.clone();
    bad.layers[2].page = 2;
    assert!(prepare_overlay_stack(&engine, bad, &[None; 3], &cancel).is_err());
    bad = opts.clone();
    bad.layers[2].alignment = OverlayAlignment::TwoPoints {
        base: [[10.0, 10.0], [201.0, 10.0]],
        revision: [[10.0, 10.0], [30.0, 10.0]],
    };
    assert!(prepare_overlay_stack(&engine, bad, &[None; 3], &cancel).is_err());
    bad = opts.clone();
    bad.layers[2].alignment = OverlayAlignment::TwoPoints {
        base: [[10.0, 10.0], [30.0, 10.0]],
        revision: [[10.0, 10.0], [30.0, 161.0]],
    };
    assert!(prepare_overlay_stack(&engine, bad, &[None; 3], &cancel).is_err());
    let oversized = temp.path().join("oversized.pdf");
    std::fs::File::create(&oversized)
        .unwrap()
        .set_len(sunipdf_core::local_review::MAX_FILE_BYTES + 1)
        .unwrap();
    bad = opts.clone();
    bad.layers[2].path = oversized;
    assert!(prepare_overlay_stack(&engine, bad, &[None; 3], &cancel).is_err());
    cancel.store(true, Ordering::Relaxed);
    assert!(matches!(
        prepare_overlay_stack(&engine, opts.clone(), &[None; 3], &cancel),
        Err(EngineError::Cancelled)
    ));
    assert!(matches!(
        preview.export_png(&cancel),
        Err(EngineError::Cancelled)
    ));
    assert!(matches!(
        preview.export_json(&cancel),
        Err(EngineError::Cancelled)
    ));
    cancel.store(false, Ordering::Relaxed);
    std::fs::remove_file(&paths[2]).unwrap();
    assert!(preview.export_png(&cancel).is_err());
    assert!(prepare_overlay_stack(&engine, opts, &[None; 3], &cancel).is_err());
}

#[test]
fn stack_checks_copy_permission_and_password_for_every_hidden_layer_without_retaining_credentials()
{
    let engine = Engine::new().expect("PDFium is required");
    let temp = tempfile::tempdir().unwrap();
    let paths: Vec<_> = (0..4)
        .map(|index| temp.path().join(format!("layer-{index}.pdf")))
        .collect();
    let original = fixture(0, [0, 0, 200, 160], "0 g 40 40 30 20 re f");
    let cancel = AtomicBool::new(false);
    for aes256 in [false, true] {
        let passwords: [String; 2] = std::array::from_fn(|_| {
            let mut random = [0u8; 16];
            getrandom::fill(&mut random).unwrap();
            random.iter().map(|byte| format!("{byte:02x}")).collect()
        });
        for allow_copy in [false, true] {
            let protected = security::protect(
                &original,
                None,
                &Protection {
                    user_password: passwords[0].clone(),
                    owner_password: passwords[1].clone(),
                    allow_copy,
                    allow_modify: false,
                    allow_annotate: false,
                    aes256,
                    ..Protection::default()
                },
            )
            .unwrap();
            for hidden_index in 0..4 {
                for (index, path) in paths.iter().enumerate() {
                    std::fs::write(
                        path,
                        if index == hidden_index {
                            &protected
                        } else {
                            &original
                        },
                    )
                    .unwrap();
                }
                let mut opts = options(&paths);
                opts.layers[hidden_index].visible = false;
                for password in &passwords {
                    let mut borrowed = [None; 4];
                    borrowed[hidden_index] = Some(password.as_str());
                    let result = prepare_overlay_stack(&engine, opts.clone(), &borrowed, &cancel);
                    if allow_copy {
                        let preview = result.unwrap();
                        let report = preview.export_json(&cancel).unwrap().to_string();
                        let debug = format!("{preview:?}");
                        for secret in &passwords {
                            assert!(!report.contains(secret));
                            assert!(!debug.contains(secret));
                        }
                        assert!(!preview.export_png(&cancel).unwrap().is_empty());
                    } else {
                        assert!(matches!(result, Err(EngineError::SecurityRestricted)));
                    }
                }
                assert!(matches!(
                    prepare_overlay_stack(&engine, opts, &[None; 4], &cancel),
                    Err(EngineError::PasswordRequired)
                ));
                assert_eq!(std::fs::read(&paths[hidden_index]).unwrap(), protected);
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn stack_refuses_selected_symlink_retarget_even_when_replacement_bytes_match() {
    let engine = Engine::new().expect("PDFium is required");
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let first = root.join("first.pdf");
    let second = root.join("second.pdf");
    let alias = root.join("selected.pdf");
    let bytes = fixture(0, [0, 0, 200, 160], "0 g 40 40 30 20 re f");
    std::fs::write(&first, &bytes).unwrap();
    std::fs::write(&second, &bytes).unwrap();
    std::os::unix::fs::symlink(&first, &alias).unwrap();
    let cancel = AtomicBool::new(false);
    let preview = prepare_overlay_stack(
        &engine,
        options(&[first.clone(), alias.clone()]),
        &[None; 2],
        &cancel,
    )
    .unwrap();
    std::fs::remove_file(&alias).unwrap();
    std::os::unix::fs::symlink(&second, &alias).unwrap();
    assert!(preview.revalidate_sources(&cancel).is_err());
    assert!(preview.export_json(&cancel).is_err());
    assert!(preview.export_png(&cancel).is_err());
    assert_eq!(std::fs::read(first).unwrap(), bytes);
}
