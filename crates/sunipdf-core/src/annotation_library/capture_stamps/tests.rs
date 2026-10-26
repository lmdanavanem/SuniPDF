use super::*;

struct Fixture {
    pdf: lopdf::Document,
    annotation: Dictionary,
    geo: PageGeometry,
    ap: lopdf::ObjectId,
    image: lopdf::ObjectId,
    mask: lopdf::ObjectId,
}

impl Fixture {
    fn capture(&self) -> Result<(LibraryPrimitive, String), EngineError> {
        capture(&self.pdf, &self.annotation, &self.geo)
    }
    fn stream(&mut self, id: lopdf::ObjectId) -> &mut Stream {
        self.pdf
            .get_object_mut(id)
            .unwrap()
            .as_stream_mut()
            .unwrap()
    }
    fn resources(&mut self) -> &mut Dictionary {
        self.stream(self.ap)
            .dict
            .get_mut(b"Resources")
            .unwrap()
            .as_dict_mut()
            .unwrap()
    }
}

fn fixture(name: &str, compressed: bool) -> Fixture {
    let geo = PageGeometry {
        bbox: PointRect {
            left: 100.0,
            bottom: 50.0,
            right: 500.0,
            top: 650.0,
        },
        rotation: crate::Rotation::R270,
    };
    let rect = PointRect {
        left: 20.0,
        bottom: 40.0,
        right: 140.0,
        top: 100.0,
    };
    let mut pdf = lopdf::Document::with_version("1.7");
    let mut mask = Stream::new(image_dictionary(2, 1, true), vec![255, 128]);
    if compressed {
        mask.compress().unwrap();
    }
    let mask = pdf.add_object(mask);
    let mut image = Stream::new(
        image_dictionary(2, 1, false),
        vec![220, 30, 40, 20, 50, 230],
    );
    image.dict.set("SMask", mask);
    if compressed {
        image.compress().unwrap();
    }
    let image = pdf.add_object(image);
    let image_stamp = name == "SuniPDFImage";
    let mut ap = crate::annot_write::ap_stream(
        &if image_stamp {
            crate::image_stamp::image_ap(120.0, 60.0, 2, 1)
        } else {
            crate::appearance::stamp_ap(120.0, 60.0, name)
        },
        &geo,
    );
    if image_stamp {
        ap.dict
            .get_mut(b"Resources")
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("XObject", dictionary! { "Image" => image });
    }
    if compressed {
        ap.compress().unwrap();
    }
    let ap = pdf.add_object(ap);
    let mut annotation =
        crate::annot_rich::rich_base("Stamp", rect, &geo, Some("Original author")).unwrap();
    annotation.set("Name", Object::Name(name.as_bytes().to_vec()));
    annotation.set(
        "Contents",
        lopdf::text_string(if image_stamp {
            "Custom image stamp"
        } else {
            name
        }),
    );
    annotation.set("AP", dictionary! { "N" => ap });
    Fixture {
        pdf,
        annotation,
        geo,
        ap,
        image,
        mask,
    }
}

#[test]
fn standard_stamps_match_full_native_appearance_and_never_enter_original_capture() {
    for (name, stamp) in [
        ("Approved", LibraryStamp::Approved),
        ("Draft", LibraryStamp::Draft),
        ("Confidential", LibraryStamp::Confidential),
        ("Rejected", LibraryStamp::Rejected),
    ] {
        for compressed in [false, true] {
            let f = fixture(name, compressed);
            assert_eq!(
                f.capture().unwrap().0,
                LibraryPrimitive::Stamp {
                    rect: [20.0, 40.0, 140.0, 100.0],
                    stamp
                }
            );
            assert!(super::super::capture_primitive(&f.pdf, &f.annotation, &f.geo).is_err());
        }
    }
    type Mutation = fn(&mut Fixture);
    let mutations: &[(&str, Mutation)] = &[
        ("custom contents", |f| {
            f.annotation
                .set("Contents", lopdf::text_string("Private text"))
        }),
        ("unrecognized name", |f| f.annotation.set("Name", "Expired")),
        ("external identity", |f| {
            f.annotation.set("NM", lopdf::text_string("Other-editor"))
        }),
        ("action", |f| {
            f.annotation.set(
                "A",
                dictionary! {"S" => "JavaScript", "JS" => Object::string_literal("test")},
            )
        }),
        ("review history", |f| {
            f.annotation.set("StateModel", lopdf::text_string("Review"))
        }),
        ("reply", |f| f.annotation.set("IRT", f.ap)),
        ("metadata", |f| f.annotation.set("Metadata", dictionary! {})),
        ("outline provenance", |f| {
            f.annotation.set("SuniPDFRevisionOutline", dictionary! {})
        }),
        ("custom AP", |f| {
            let ap = f.stream(f.ap);
            ap.set_content(b"q Q".to_vec());
        }),
        ("altered AP matrix", |f| {
            f.stream(f.ap).dict.set(
                "Matrix",
                vec![1.into(), 0.into(), 0.into(), 1.into(), 0.into(), 0.into()],
            )
        }),
        ("alternate AP", |f| {
            f.annotation
                .get_mut(b"AP")
                .unwrap()
                .as_dict_mut()
                .unwrap()
                .set("R", f.ap)
        }),
        ("unused resource", |f| {
            f.resources().set("Pattern", dictionary! {})
        }),
        ("custom font", |f| {
            f.resources()
                .get_mut(b"Font")
                .unwrap()
                .as_dict_mut()
                .unwrap()
                .get_mut(b"Helv")
                .unwrap()
                .as_dict_mut()
                .unwrap()
                .set("BaseFont", "Courier")
        }),
        ("font metadata", |f| {
            f.resources()
                .get_mut(b"Font")
                .unwrap()
                .as_dict_mut()
                .unwrap()
                .get_mut(b"Helv")
                .unwrap()
                .as_dict_mut()
                .unwrap()
                .set("Metadata", Object::Null)
        }),
        ("opacity", |f| {
            f.resources()
                .get_mut(b"ExtGState")
                .unwrap()
                .as_dict_mut()
                .unwrap()
                .get_mut(b"GS")
                .unwrap()
                .as_dict_mut()
                .unwrap()
                .set("ca", 0.4)
        }),
        ("predictor parameters", |f| {
            f.stream(f.ap)
                .dict
                .set("DecodeParms", dictionary! {"Predictor" => 1})
        }),
        ("unknown filter", |f| {
            f.stream(f.ap).dict.set("Filter", "ASCIIHexDecode")
        }),
        ("filter array", |f| {
            f.stream(f.ap)
                .dict
                .set("Filter", vec![Object::Name(b"FlateDecode".to_vec())])
        }),
    ];
    for (label, mutate) in mutations {
        let mut f = fixture("Approved", false);
        mutate(&mut f);
        assert!(f.capture().is_err(), "accepted {label}");
    }
}

#[test]
fn fractional_native_rectangles_use_verified_bbox_dimensions_at_every_rotation() {
    let rect = PointRect {
        left: 20.1,
        bottom: 30.2,
        right: 148.2,
        top: 91.3,
    };
    let (w, h) = (rect.right - rect.left, rect.top - rect.bottom);
    for rotation in [
        crate::Rotation::R0,
        crate::Rotation::R90,
        crate::Rotation::R180,
        crate::Rotation::R270,
    ] {
        for name in [
            "Approved",
            "Draft",
            "Confidential",
            "Rejected",
            "SuniPDFImage",
        ] {
            let mut f = fixture(name, false);
            f.geo.rotation = rotation;
            let user_rect = f.geo.rect_to_user(rect);
            f.annotation
                .set("Rect", crate::annot_write::rect_obj(user_rect));
            let mut ap = crate::annot_write::ap_stream(
                &if name == "SuniPDFImage" {
                    crate::image_stamp::image_ap(w, h, 2, 1)
                } else {
                    crate::appearance::stamp_ap(w, h, name)
                },
                &f.geo,
            );
            if name == "SuniPDFImage" {
                ap.dict
                    .get_mut(b"Resources")
                    .unwrap()
                    .as_dict_mut()
                    .unwrap()
                    .set("XObject", dictionary! { "Image" => f.image });
            }
            f.pdf.objects.insert(f.ap, Object::Stream(ap));
            assert!(
                f.capture().is_ok(),
                "native fractional {name} at {rotation:?}"
            );

            // An internally canonical AP with an unrelated size must still fail:
            // content/resource equality alone is not proof of the Rect geometry.
            let mut enlarged = crate::annot_write::ap_stream(
                &if name == "SuniPDFImage" {
                    crate::image_stamp::image_ap(w + 0.01, h, 2, 1)
                } else {
                    crate::appearance::stamp_ap(w + 0.01, h, name)
                },
                &f.geo,
            );
            if name == "SuniPDFImage" {
                enlarged
                    .dict
                    .get_mut(b"Resources")
                    .unwrap()
                    .as_dict_mut()
                    .unwrap()
                    .set("XObject", dictionary! { "Image" => f.image });
            }
            f.pdf.objects.insert(f.ap, Object::Stream(enlarged));
            assert!(
                f.capture().is_err(),
                "geometry mismatch accepted for {name} at {rotation:?}"
            );
        }
    }
    assert!(
        !native_span_matches(120.0, 248.0, 128.0001),
        "sub-millipoint changes beyond the derived ULP interval are refused"
    );
    assert!(
        !native_span_matches(1_000_000_000.0, 1_000_000_064.0, 40.0),
        "coarse coordinates cannot authorize a broad geometry tolerance"
    );
}

#[test]
fn image_capture_preserves_pixels_and_rejects_custom_resource_graphs() {
    for compressed in [false, true] {
        let f = fixture("SuniPDFImage", compressed);
        let (LibraryPrimitive::Image { png, opacity, rect }, _) = f.capture().unwrap() else {
            panic!("missing image")
        };
        assert_eq!(opacity, 1.0);
        assert_eq!(rect, [20.0, 40.0, 140.0, 100.0]);
        let rgba = image::load_from_memory(&png.png_bytes().unwrap())
            .unwrap()
            .into_rgba8();
        assert_eq!(rgba.into_raw(), [220, 30, 40, 255, 20, 50, 230, 128]);
        assert!(super::super::capture_primitive(&f.pdf, &f.annotation, &f.geo).is_err());
    }
    type Mutation = fn(&mut Fixture);
    let mutations: &[(&str, Mutation)] = &[
        ("aspect fit", |f| {
            f.stream(f.ap)
                .set_content(b"q /GS gs 120 0 0 59 0 0 cm /Image Do Q\n".to_vec());
        }),
        ("extra XObject", |f| {
            let id = f.image;
            f.resources()
                .get_mut(b"XObject")
                .unwrap()
                .as_dict_mut()
                .unwrap()
                .set("Extra", id);
        }),
        ("unused font", |f| f.resources().set("Font", dictionary! {})),
        ("image mask", |f| {
            f.stream(f.image).dict.set("ImageMask", true)
        }),
        ("alternate color space", |f| {
            f.stream(f.image).dict.set("ColorSpace", "DeviceCMYK")
        }),
        ("decode mapping", |f| {
            f.stream(f.image)
                .dict
                .set("Decode", vec![0.into(), 1.into()])
        }),
        ("pixel depth", |f| {
            f.stream(f.image).dict.set("BitsPerComponent", 16)
        }),
        ("noninteger dimension", |f| {
            f.stream(f.image).dict.set("Width", 2.0)
        }),
        ("unknown filter", |f| {
            f.stream(f.image).dict.set("Filter", "DCTDecode")
        }),
        ("predictor", |f| {
            f.stream(f.image)
                .dict
                .set("DecodeParms", dictionary! { "Predictor" => 1 })
        }),
        ("metadata", |f| {
            f.stream(f.image).dict.set("Metadata", Object::Null)
        }),
        ("interpolation", |f| {
            f.stream(f.image).dict.set("Interpolate", true)
        }),
        ("short RGB", |f| {
            f.stream(f.image).set_content(vec![1, 2, 3])
        }),
        ("trailing RGB", |f| {
            f.stream(f.image).set_content(vec![1; 7])
        }),
        ("mask dimensions", |f| {
            f.stream(f.mask).dict.set("Height", 2)
        }),
        ("mask color space", |f| {
            f.stream(f.mask).dict.set("ColorSpace", "DeviceRGB")
        }),
        ("mask depth", |f| {
            f.stream(f.mask).dict.set("BitsPerComponent", 1)
        }),
        ("recursive mask", |f| {
            let id = f.mask;
            f.stream(id).dict.set("SMask", id);
        }),
        ("short mask", |f| f.stream(f.mask).set_content(vec![255])),
        ("redundant mask", |f| {
            f.stream(f.mask).set_content(vec![255, 255])
        }),
        ("invisible image", |f| {
            f.stream(f.mask).set_content(vec![0, 0]);
            f.stream(f.image).set_content(vec![0; 6]);
        }),
        ("hidden RGB", |f| f.stream(f.mask).set_content(vec![255, 0])),
        ("reference cycle", |f| {
            f.pdf.objects.insert(f.image, Object::Reference(f.image));
        }),
    ];
    for (label, mutate) in mutations {
        let mut f = fixture("SuniPDFImage", false);
        mutate(&mut f);
        assert!(f.capture().is_err(), "accepted {label}");
    }
}

#[test]
fn compressed_rgb_and_soft_mask_capture_is_canonical_and_pixel_changes_are_fingerprinted() {
    let mut f = fixture("SuniPDFImage", false);
    let image = f.stream(f.image);
    image.dict.set("Width", 32);
    image.dict.set("Height", 32);
    image.set_content([220, 30, 40].repeat(1024));
    image.compress().unwrap();
    assert!(image.dict.has(b"Filter"));
    let mask = f.stream(f.mask);
    mask.dict.set("Width", 32);
    mask.dict.set("Height", 32);
    mask.set_content(vec![128; 1024]);
    mask.compress().unwrap();
    assert!(mask.dict.has(b"Filter"));
    let content = crate::image_stamp::image_ap(120.0, 60.0, 32, 32).content;
    f.stream(f.ap).set_content(content);
    let (primitive, hash) = f.capture().unwrap();
    let LibraryPrimitive::Image { png, opacity, .. } = primitive else {
        panic!("missing image")
    };
    assert_eq!(opacity, 1.0);
    assert_eq!(png.dimensions(), (32, 32));
    assert!(image::load_from_memory(&png.png_bytes().unwrap())
        .unwrap()
        .into_rgba8()
        .pixels()
        .all(|pixel| pixel.0 == [220, 30, 40, 128]));
    f.stream(f.image).decompress().unwrap();
    f.stream(f.mask).decompress().unwrap();
    assert_eq!(
        f.capture().unwrap().1,
        hash,
        "lossless storage encoding does not change identity"
    );
    f.stream(f.image).content[0] = 219;
    assert_ne!(
        f.capture().unwrap().1,
        hash,
        "RGB pixels are part of the selection fingerprint"
    );
    f.stream(f.image).content[0] = 220;
    f.stream(f.mask).content[0] = 127;
    assert_ne!(
        f.capture().unwrap().1,
        hash,
        "alpha pixels are part of the selection fingerprint"
    );
}

#[test]
fn image_capture_bounds_dimensions_decompression_and_complete_graph_bytes() {
    for (width, height) in [(4097, 1), (1, 4097), (2001, 2000), (0, 1)] {
        let mut f = fixture("SuniPDFImage", false);
        f.stream(f.image).dict.set("Width", width);
        f.stream(f.image).dict.set("Height", height);
        assert!(f.capture().is_err());
    }
    let mut f = fixture("SuniPDFImage", false);
    let stream = f.stream(f.image);
    stream.set_content(vec![0; 2 * 1024 * 1024 + 1]);
    stream.compress().unwrap();
    assert!(
        stream.content.len() < 4096,
        "the bomb must remain small on disk"
    );
    assert!(
        f.capture().is_err(),
        "decompressed data cannot exceed the canonical capture budget"
    );

    // RGB fits on its own; adding the otherwise-valid mask crosses the unchanged 2 MiB limit.
    let mut f = fixture("SuniPDFImage", false);
    let width = 750;
    let height = 750;
    let image = f.stream(f.image);
    image.dict.set("Width", width);
    image.dict.set("Height", height);
    image.set_content(vec![30; width as usize * height as usize * 3]);
    image.compress().unwrap();
    let mask = f.stream(f.mask);
    mask.dict.set("Width", width);
    mask.dict.set("Height", height);
    mask.set_content(vec![128; width as usize * height as usize]);
    mask.compress().unwrap();
    assert!(
        f.capture().is_err(),
        "mask and RGB share one canonical budget"
    );
}
