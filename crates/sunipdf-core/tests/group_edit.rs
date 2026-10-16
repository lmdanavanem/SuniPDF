use std::path::Path;
use sunipdf_core::{
    security, Engine, ObjectInfo, PageMargins, PointRect, Protection, RenderedPage, ShapeKind,
    ShapeStyle, TextStyle,
};

const PDF: &[u8] = include_bytes!("fixtures/two-pages.pdf");

fn rect(left: f32, bottom: f32, right: f32, top: f32) -> PointRect {
    PointRect {
        left,
        bottom,
        right,
        top,
    }
}

fn union(objects: &[ObjectInfo]) -> PointRect {
    objects
        .iter()
        .fold(rect(f32::MAX, f32::MAX, f32::MIN, f32::MIN), |u, o| {
            rect(
                u.left.min(o.bounds.left),
                u.bottom.min(o.bounds.bottom),
                u.right.max(o.bounds.right),
                u.top.max(o.bounds.top),
            )
        })
}

fn center(r: PointRect) -> (f32, f32) {
    ((r.left + r.right) / 2.0, (r.bottom + r.top) / 2.0)
}

fn pixel(img: &RenderedPage, p: (f32, f32)) -> [u8; 3] {
    let offset =
        (((img.height as f32 - p.1).floor() as u32 * img.width + p.0.floor() as u32) * 4) as usize;
    img.rgba[offset..offset + 3].try_into().unwrap()
}

fn assert_quads(
    before: &[ObjectInfo],
    after: &[ObjectInfo],
    transform: impl Fn((f32, f32)) -> (f32, f32),
) {
    for (old, new) in before.iter().zip(after) {
        assert_eq!(old.kind, new.kind);
        for point in old.quad {
            let expected = transform(point);
            assert!(
                new.quad
                    .iter()
                    .any(|actual| (actual.0 - expected.0).abs() < 0.2
                        && (actual.1 - expected.1).abs() < 0.2),
                "expected {expected:?} in {:?}",
                new.quad
            );
        }
    }
}

#[test]
fn group_transforms_and_mouse_crop_survive_rotated_cropped_encrypted_save_undo_redo() {
    let engine = Engine::new().unwrap();
    let dir = std::env::temp_dir().join(format!("sunipdf-group-edit-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for rotation in [0, 90, 180, 270] {
        for encrypted in [false, true] {
            let mut pdf = lopdf::Document::load_mem(PDF).unwrap();
            let page = pdf.get_pages()[&1];
            let page = pdf.get_dictionary_mut(page).unwrap();
            page.set("Rotate", rotation);
            page.set(
                "CropBox",
                vec![100.into(), 50.into(), 500.into(), 650.into()],
            );
            let mut bytes = Vec::new();
            pdf.save_to(&mut bytes).unwrap();
            if encrypted {
                bytes = security::protect(
                    &bytes,
                    None,
                    &Protection {
                        user_password: "user".into(),
                        owner_password: "owner".into(),
                        ..Default::default()
                    },
                )
                .unwrap();
            }
            let password = encrypted.then_some("user");
            let mut doc = engine
                .open_bytes(bytes, password, Path::new("group-source.pdf"))
                .unwrap();
            let indices: Vec<_> = doc
                .page_objects(0)
                .unwrap()
                .iter()
                .map(|o| o.index)
                .collect();
            doc.delete_objects(0, &indices).unwrap();
            doc.add_shape(
                0,
                ShapeKind::Rectangle,
                rect(80.0, 100.0, 120.0, 140.0),
                &ShapeStyle {
                    stroke_rgb: (255, 0, 0),
                    fill_rgb: Some((255, 0, 0)),
                    stroke_width: 0.1,
                    opacity: 1.0,
                },
            )
            .unwrap();
            doc.add_image_data(
                0,
                &image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
                    16,
                    10,
                    image::Rgb([0, 0, 255]),
                )),
                200.0,
                110.0,
                40.0,
            )
            .unwrap();
            doc.add_text(0, 145.0, 155.0, "GROUP", &TextStyle::default())
                .unwrap();
            let before = doc.page_objects(0).unwrap();
            let indices: Vec<_> = before.iter().map(|o| o.index).collect();
            assert_eq!(before.len(), 3);
            let before_pixels = doc.render_page(0, 1.0).unwrap().rgba;
            let undo = doc.snapshot().unwrap();
            let c = center(union(&before));
            let rotate = |p: (f32, f32)| (c.0 + p.1 - c.1, c.1 - p.0 + c.0);
            let mut duplicates = indices.clone();
            duplicates.push(indices[0]);
            doc.rotate_objects(0, &duplicates, 90.0).unwrap();
            let rotated = doc.page_objects(0).unwrap();
            assert_quads(&before, &rotated, rotate);
            let img = doc.render_page(0, 1.0).unwrap();
            assert_eq!(pixel(&img, rotate(center(before[0].bounds))), [255, 0, 0]);
            assert_eq!(pixel(&img, rotate(center(before[1].bounds))), [0, 0, 255]);

            let current = union(&rotated);
            let target = rect(
                100.0,
                100.0,
                100.0 + (current.right - current.left) * 1.25,
                100.0 + (current.top - current.bottom) * 0.75,
            );
            let scale = |p: (f32, f32)| {
                (
                    100.0 + (p.0 - current.left) * 1.25,
                    100.0 + (p.1 - current.bottom) * 0.75,
                )
            };
            doc.resize_objects(0, &indices, target).unwrap();
            let resized = doc.page_objects(0).unwrap();
            assert_quads(&rotated, &resized, scale);
            let img = doc.render_page(0, 1.0).unwrap();
            assert_eq!(pixel(&img, scale(center(rotated[0].bounds))), [255, 0, 0]);
            assert_eq!(pixel(&img, scale(center(rotated[1].bounds))), [0, 0, 255]);
            assert!(doc.page_text(0).unwrap().contains("GROUP"));
            let redo = doc.snapshot().unwrap();
            doc.restore(undo.clone()).unwrap();
            assert_eq!(doc.render_page(0, 1.0).unwrap().rgba, before_pixels);
            doc.restore(redo).unwrap();
            assert_quads(&resized, &doc.page_objects(0).unwrap(), |p| p);

            let before_bad = doc.page_objects(0).unwrap();
            for bad in [
                rect(f32::NAN, 0.0, 100.0, 100.0),
                rect(0.0, 0.0, f32::INFINITY, 100.0),
                rect(20.0, 20.0, 10.0, 50.0),
                rect(-f32::MAX, 0.0, f32::MAX, 50.0),
            ] {
                assert!(doc.resize_objects(0, &indices, bad).is_err());
            }
            assert!(doc.rotate_objects(0, &indices, f32::NAN).is_err());
            assert!(doc
                .rotate_objects(0, &[indices[0], usize::MAX], 30.0)
                .is_err());
            assert!(doc.resize_objects(0, &[], target).is_err());
            assert_eq!(
                doc.page_objects(0).unwrap(),
                before_bad,
                "invalid input must not move a subset"
            );

            let size = doc.page_size(0).unwrap();
            let margins = PageMargins {
                left: 20.0,
                right: 30.0,
                top: 40.0,
                bottom: 10.0,
            };
            let before_crop = doc.snapshot().unwrap();
            let cropped = doc.crop_page(0, margins).unwrap();
            assert_eq!(cropped, (size.width_pt - 50.0, size.height_pt - 50.0));
            let after_crop = doc.page_objects(0).unwrap();
            assert_quads(&resized, &after_crop, |p| (p.0 - 20.0, p.1 - 10.0));
            let img = doc.render_page(0, 1.0).unwrap();
            assert_eq!(
                pixel(
                    &img,
                    (
                        center(resized[0].bounds).0 - 20.0,
                        center(resized[0].bounds).1 - 10.0
                    )
                ),
                [255, 0, 0]
            );
            let cropped_pixels = img.rgba;
            let out = dir.join(format!("group-{rotation}-{encrypted}.pdf"));
            doc.save_as(&out).unwrap();
            assert_eq!(
                lopdf::Document::load(&out).unwrap().is_encrypted(),
                encrypted
            );
            let reopened = engine.open(&out, password).unwrap();
            assert_eq!(reopened.render_page(0, 1.0).unwrap().rgba, cropped_pixels);
            assert!(reopened.page_text(0).unwrap().contains("GROUP"));
            doc.restore(before_crop).unwrap();
            assert_quads(&resized, &doc.page_objects(0).unwrap(), |p| p);
        }
    }
    std::fs::remove_dir_all(dir).unwrap();
}
