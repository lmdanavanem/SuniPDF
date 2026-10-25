use lopdf::{dictionary, Document as Pdf, Object, Stream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use sunipdf_core::revision_overlay::recipe::{self, Manifest};
use sunipdf_core::revision_overlay::stack::{
    prepare_overlay_stack, OverlayStackLayer, OverlayStackOptions,
};
use sunipdf_core::revision_overlay::{OverlayAlignment, OverlayTransform};
use sunipdf_core::{security, Engine, EngineError, Protection};

fn fixture(rotation: i64, ink: i64) -> Vec<u8> {
    let mut pdf = Pdf::with_version("1.7");
    let pages = pdf.new_object_id();
    let mut kids = Vec::new();
    for page in 0..2 {
        let content = format!("0 g {} {} 30 20 re f", 120 + ink + page * 7, 80 + page * 9);
        let stream = pdf.add_object(Stream::new(dictionary! {}, content.into_bytes()));
        let page = pdf.add_object(dictionary! {
            "Type"=>"Page", "Parent"=>pages, "MediaBox"=>vec![0.into(),0.into(),600.into(),800.into()],
            "CropBox"=>vec![100.into(),50.into(),300.into(),210.into()], "Rotate"=>rotation,
            "Resources"=>dictionary!{}, "Contents"=>stream,
        });
        kids.push(Object::Reference(page));
    }
    pdf.objects.insert(
        pages,
        dictionary! {"Type"=>"Pages", "Count"=>2, "Kids"=>kids}.into(),
    );
    let root = pdf.add_object(dictionary! {"Type"=>"Catalog", "Pages"=>pages});
    pdf.trailer.set("Root", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

fn options(paths: &[PathBuf]) -> OverlayStackOptions {
    let mut layers: Vec<_> = paths
        .iter()
        .enumerate()
        .map(|(index, path)| {
            let mut layer = OverlayStackLayer::new(path.clone(), index % 2 + 1, index);
            layer.color = [
                20 + index as u8 * 40,
                180 - index as u8 * 30,
                80 + index as u8 * 20,
            ];
            layer.opacity = 0.4 + index as f32 * 0.15;
            if index > 0 {
                layer.alignment = OverlayAlignment::Manual {
                    transform: OverlayTransform {
                        translation: [4.25 * index as f64, -3.5],
                        rotation_degrees: 15.0,
                        scale: 0.9,
                    },
                };
            }
            layer
        })
        .collect();
    if layers.len() >= 3 {
        layers[2].visible = false;
        layers[2].alignment = OverlayAlignment::TwoPoints {
            base: [[10.0, 10.0], [80.0, 10.0]],
            revision: [[20.0, 20.0], [80.0, 20.0]],
        };
    }
    OverlayStackOptions {
        layers,
        long_side: 128,
    }
}

fn loaded_preview(
    engine: &Engine,
    path: &Path,
    passwords: &[Option<&str>],
) -> sunipdf_core::revision_overlay::stack::OverlayStackPreview {
    let loaded = recipe::read_regular(path).unwrap();
    recipe::require_desktop(&loaded.manifest).unwrap();
    recipe::prepare(
        engine,
        &loaded.manifest,
        loaded.path.parent().unwrap(),
        passwords,
        &AtomicBool::new(false),
    )
    .unwrap()
}

#[test]
fn source_bound_overlay_setups_round_trip_and_fail_closed() {
    // PDFium is initialized once; all native cases run sequentially in this binary.
    let engine = Engine::new().expect("PDFium is required");
    round_trip_layers_styles_pages_alignments_and_rotations(&engine);
    stale_hidden_missing_cancelled_and_existing_outputs_are_refused(&engine);
    encrypted_sources_require_credentials_again_without_saving_secrets(&engine);
    #[cfg(unix)]
    selected_alias_retarget_is_refused_before_setup_publication(&engine);
}

fn round_trip_layers_styles_pages_alignments_and_rotations(engine: &Engine) {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let paths: Vec<_> = (0..4)
        .map(|index| root.join(format!("étage-{index}.pdf")))
        .collect();
    let cancel = AtomicBool::new(false);
    for rotation in [0, 90, 180, 270] {
        let sources: Vec<_> = (0..4).map(|index| fixture(rotation, index * 11)).collect();
        for (path, bytes) in paths.iter().zip(&sources) {
            std::fs::write(path, bytes).unwrap();
        }
        for count in 2..=4 {
            let options = options(&paths[..count]);
            let preview =
                prepare_overlay_stack(engine, options.clone(), &vec![None; count], &cancel)
                    .unwrap();
            let output = root.join(format!("setup-{rotation}-{count}.json"));
            let saved = recipe::save_new(&preview, &output, &cancel).unwrap();
            recipe::require_desktop(&saved).unwrap();
            assert_eq!(saved.version, 1);
            assert_eq!(saved.long_side, options.long_side);
            assert_eq!(saved.layers.len(), count);
            for (index, layer) in saved.layers.iter().enumerate() {
                assert!(
                    !layer.path.is_absolute(),
                    "contained sources should remain portable"
                );
                assert_eq!(layer.page, options.layers[index].page);
                assert_eq!(layer.color, Some(options.layers[index].color));
                assert_eq!(layer.opacity, Some(options.layers[index].opacity));
                assert_eq!(layer.visible, Some(options.layers[index].visible));
                assert_eq!(layer.alignment, options.layers[index].alignment);
                assert_eq!(
                    layer.expected_sha256.as_deref(),
                    preview.source_sha256(index)
                );
                assert!(layer.password_env.is_none());
            }
            let reopened = loaded_preview(engine, &output, &vec![None; count]);
            assert_eq!(reopened.image().rgba, preview.image().rgba);
            assert_eq!(
                reopened.export_png(&cancel).unwrap(),
                preview.export_png(&cancel).unwrap()
            );
            for index in 0..count {
                assert_eq!(reopened.transform(index), preview.transform(index));
                assert_eq!(
                    reopened.layer_image(index).unwrap().rgba,
                    preview.layer_image(index).unwrap().rgba
                );
            }
            let mut uppercase = saved.clone();
            for layer in &mut uppercase.layers {
                layer
                    .expected_sha256
                    .as_mut()
                    .unwrap()
                    .make_ascii_uppercase();
            }
            assert_eq!(
                recipe::prepare(engine, &uppercase, &root, &vec![None; count], &cancel)
                    .unwrap()
                    .image()
                    .rgba,
                preview.image().rgba
            );
            let output_bytes = std::fs::read(&output).unwrap();
            assert!(matches!(
                recipe::save_new(&preview, &output, &cancel),
                Err(EngineError::AlreadyExists(_))
            ));
            assert_eq!(std::fs::read(&output).unwrap(), output_bytes);
            for (path, bytes) in paths.iter().zip(&sources) {
                assert_eq!(std::fs::read(path).unwrap(), *bytes);
            }
        }
    }
    let other = tempfile::tempdir().unwrap();
    let preview = prepare_overlay_stack(engine, options(&paths[..2]), &[None; 2], &cancel).unwrap();
    let output = other.path().join("elsewhere.json");
    let manifest = recipe::save_new(&preview, &output, &cancel).unwrap();
    assert!(manifest.layers.iter().all(|layer| layer.path.is_absolute()));
    assert_eq!(
        loaded_preview(engine, &output, &[None; 2]).image().rgba,
        preview.image().rgba
    );
    assert!(preview.source_sha256(4).is_none());
}

fn stale_hidden_missing_cancelled_and_existing_outputs_are_refused(engine: &Engine) {
    let directory = tempfile::tempdir().unwrap();
    let paths: Vec<_> = (0..3)
        .map(|index| directory.path().join(format!("source-{index}.pdf")))
        .collect();
    let original = fixture(0, 0);
    for path in &paths {
        std::fs::write(path, &original).unwrap();
    }
    let cancel = AtomicBool::new(false);
    let preview = prepare_overlay_stack(engine, options(&paths), &[None; 3], &cancel).unwrap();
    let setup = directory.path().join("setup.json");
    let saved = recipe::save_new(&preview, &setup, &cancel).unwrap();
    let expected: Vec<_> = saved
        .layers
        .iter()
        .map(|layer| layer.expected_sha256.clone())
        .collect();
    assert!(recipe::verify_expected_hashes(&preview, &expected[..2], &cancel).is_err());
    assert!(recipe::save_new(&preview, &paths[0], &cancel).is_err());
    assert_eq!(std::fs::read(&paths[0]).unwrap(), original);
    let alias = directory.path().join("source-alias.pdf");
    std::fs::hard_link(&paths[0], &alias).unwrap();
    assert!(recipe::save_new(&preview, &alias, &cancel).is_err());
    assert_eq!(std::fs::read(&paths[0]).unwrap(), original);
    let cancelled = directory.path().join("cancelled.json");
    cancel.store(true, Ordering::Relaxed);
    assert!(matches!(
        recipe::save_new(&preview, &cancelled, &cancel),
        Err(EngineError::Cancelled)
    ));
    assert!(matches!(
        recipe::prepare(engine, &saved, directory.path(), &[None; 3], &cancel),
        Err(EngineError::Cancelled)
    ));
    assert!(!cancelled.exists());
    cancel.store(false, Ordering::Relaxed);
    std::fs::write(&paths[2], fixture(0, 70)).unwrap();
    let stale = directory.path().join("stale.json");
    assert!(recipe::save_new(&preview, &stale, &cancel).is_err());
    assert!(!stale.exists());
    assert!(
        recipe::prepare(engine, &saved, directory.path(), &[None; 3], &cancel)
            .unwrap_err()
            .to_string()
            .contains("expected SHA-256")
    );
    let replacement = prepare_overlay_stack(engine, options(&paths), &[None; 3], &cancel).unwrap();
    assert!(recipe::verify_expected_hashes(&replacement, &expected, &cancel).is_err());
    std::fs::remove_file(&paths[2]).unwrap();
    assert!(recipe::prepare(engine, &saved, directory.path(), &[None; 3], &cancel).is_err());
    assert!(recipe::save_new(&replacement, &stale, &cancel).is_err());
    assert!(!stale.exists());
}

fn encrypted_sources_require_credentials_again_without_saving_secrets(engine: &Engine) {
    let directory = tempfile::tempdir().unwrap();
    let paths: Vec<_> = (0..3)
        .map(|index| directory.path().join(format!("protected-{index}.pdf")))
        .collect();
    let plain = fixture(0, 0);
    let cancel = AtomicBool::new(false);
    for aes256 in [false, true] {
        for allowed in [false, true] {
            let password = || {
                let mut bytes = [0u8; 24];
                getrandom::fill(&mut bytes).unwrap();
                bytes
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            };
            let user = password();
            let owner = password();
            let encrypted = security::protect(
                &plain,
                None,
                &Protection {
                    user_password: user.clone(),
                    owner_password: owner.clone(),
                    allow_copy: allowed,
                    allow_modify: false,
                    allow_annotate: false,
                    aes256,
                    ..Protection::default()
                },
            )
            .unwrap();
            for (index, path) in paths.iter().enumerate() {
                std::fs::write(path, if index == 2 { &encrypted } else { &plain }).unwrap();
            }
            let preview = prepare_overlay_stack(
                engine,
                options(&paths),
                &[None, None, Some(user.as_str())],
                &cancel,
            );
            if !allowed {
                assert!(matches!(preview, Err(EngineError::SecurityRestricted)));
                continue;
            }
            let preview = preview.unwrap();
            let output = directory.path().join(format!("encrypted-{aes256}.json"));
            let saved = recipe::save_new(&preview, &output, &cancel).unwrap();
            let bytes = std::fs::read(&output).unwrap();
            let text = String::from_utf8(bytes).unwrap();
            for secret in [user.as_str(), owner.as_str(), "password_env", "password"] {
                assert!(!text.contains(secret));
            }
            let mut cli: Manifest = saved.clone();
            cli.layers[2].password_env = Some("LOCAL_OVERLAY_PASSWORD".into());
            assert!(recipe::require_desktop(&cli).is_err());
            assert!(matches!(
                recipe::prepare(engine, &saved, directory.path(), &[None; 3], &cancel),
                Err(EngineError::PasswordRequired)
            ));
            assert_eq!(
                loaded_preview(engine, &output, &[None, None, Some(user.as_str())])
                    .image()
                    .rgba,
                preview.image().rgba
            );
            // CLI compatibility keeps environment names as references without ever resolving them here.
            assert_eq!(
                recipe::prepare(
                    engine,
                    &cli,
                    directory.path(),
                    &[None, None, Some(user.as_str())],
                    &cancel
                )
                .unwrap()
                .image()
                .rgba,
                preview.image().rgba
            );
            assert_eq!(std::fs::read(&paths[2]).unwrap(), encrypted);
        }
    }
}

#[cfg(unix)]
fn selected_alias_retarget_is_refused_before_setup_publication(engine: &Engine) {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let first = root.join("first.pdf");
    let second = root.join("second.pdf");
    let selected = root.join("selected.pdf");
    let bytes = fixture(0, 0);
    std::fs::write(&first, &bytes).unwrap();
    std::fs::write(&second, &bytes).unwrap();
    std::os::unix::fs::symlink(&first, &selected).unwrap();
    let cancel = AtomicBool::new(false);
    let preview = prepare_overlay_stack(
        engine,
        options(&[first.clone(), selected.clone()]),
        &[None; 2],
        &cancel,
    )
    .unwrap();
    let initial = root.join("initial.json");
    let saved = recipe::save_new(&preview, &initial, &cancel).unwrap();
    assert_eq!(saved.layers[1].path, Path::new("selected.pdf"));
    std::fs::remove_file(&selected).unwrap();
    std::os::unix::fs::symlink(&second, &selected).unwrap();
    let refused = root.join("refused.json");
    assert!(recipe::save_new(&preview, &refused, &cancel).is_err());
    assert!(!refused.exists());
    assert_eq!(std::fs::read(first).unwrap(), bytes);
}
