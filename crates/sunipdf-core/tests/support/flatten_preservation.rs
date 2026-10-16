use lopdf::{dictionary, Dictionary, Object, ObjectId, Stream};

pub const PAYLOAD: &[u8] = b"SuniPDF attachment regression\0\x01\xfe\xff\n";
const NAMES: [&str; 15] = [
    "attachment",
    "popup",
    "reply",
    "screen-no-ap",
    "screen-rollover-only",
    "screen-null-normal",
    "screen-scalar-normal",
    "screen-empty-states",
    "screen-unmatched-state",
    "screen-invalid-state",
    "screen-indirect-null",
    "screen-missing-box",
    "screen-empty-box",
    "screen-empty-rect",
    "screen-invalid-rect",
];

fn annotation(subtype: &str, name: &str, page: ObjectId, flags: i64) -> Dictionary {
    dictionary! {
        "Type"=>"Annot", "Subtype"=>subtype, "P"=>page, "F"=>flags,
        "Rect"=>vec![40.into(),40.into(),80.into(),60.into()],
        "NM"=>Object::string_literal(name), "Contents"=>Object::string_literal(name),
    }
}

pub fn document() -> Vec<u8> {
    let mut pdf =
        lopdf::Document::load_mem(include_bytes!("../fixtures/acroform-links.pdf")).unwrap();
    let page = pdf.get_pages()[&1];
    let embedded = pdf.add_object(Stream::new(
        dictionary! {"Type"=>"EmbeddedFile", "Params"=>dictionary! {"Size"=>PAYLOAD.len() as i64}},
        PAYLOAD.to_vec(),
    ));
    let file = pdf.add_object(dictionary! {
        "Type"=>"Filespec", "F"=>Object::string_literal("regression.bin"),
        "UF"=>Object::string_literal("regression.bin"), "EF"=>dictionary! {"F"=>embedded},
    });
    let attachment = pdf.new_object_id();
    let popup = pdf.new_object_id();
    let reply = pdf.new_object_id();
    let mut attach = annotation("FileAttachment", "attachment", page, 0);
    attach.set("FS", file);
    attach.set("Popup", popup);
    attach.set("IRT", reply);
    let mut popup_dict = annotation("Popup", "popup", page, 2);
    popup_dict.set("Parent", attachment);
    let mut reply_dict = annotation("Screen", "reply", page, 32);
    reply_dict.set("IRT", attachment);
    reply_dict.set("FS", file);
    pdf.objects.insert(attachment, attach.into());
    pdf.objects.insert(popup, popup_dict.into());
    pdf.objects.insert(reply, reply_dict.into());
    let mut annots = annotations(&pdf, 0)
        .into_iter()
        .map(|(id, _)| Object::Reference(id.unwrap()))
        .collect::<Vec<_>>();
    annots.extend([attachment.into(), popup.into(), reply.into()]);
    annots.push(annotation("Screen", "screen-no-ap", page, 0).into());

    let appearance = pdf.add_object(Stream::new(
        dictionary! {
            "Type"=>"XObject", "Subtype"=>"Form", "FormType"=>1,
            "BBox"=>vec![0.into(),0.into(),40.into(),20.into()],
            "Resources"=>dictionary! {},
        },
        b"0 0 1 rg 0 0 40 20 re f".to_vec(),
    ));
    let null = pdf.add_object(Object::Null);
    let variants = [
        ("screen-rollover-only", dictionary! {"R"=>appearance}, None),
        ("screen-null-normal", dictionary! {"N"=>Object::Null}, None),
        ("screen-scalar-normal", dictionary! {"N"=>7}, None),
        (
            "screen-empty-states",
            dictionary! {"N"=>dictionary! {}},
            None,
        ),
        (
            "screen-unmatched-state",
            dictionary! {"N"=>dictionary! {"On"=>appearance}},
            Some("Missing"),
        ),
        (
            "screen-invalid-state",
            dictionary! {"N"=>dictionary! {"On"=>appearance, "Off"=>7}},
            Some("Off"),
        ),
        ("screen-indirect-null", dictionary! {"N"=>null}, None),
    ];
    for (name, ap, state) in variants {
        let mut screen = annotation("Screen", name, page, 0);
        screen.set("AP", ap);
        if let Some(state) = state {
            screen.set("AS", Object::Name(state.as_bytes().to_vec()));
        }
        annots.push(pdf.add_object(screen).into());
    }
    let valid_box = Object::Array(vec![0.into(), 0.into(), 40.into(), 20.into()]);
    let empty_box = Object::Array(vec![0.into(), 0.into(), 0.into(), 20.into()]);
    for (name, rect, bbox) in [
        ("screen-missing-box", None, None),
        ("screen-empty-box", None, Some(empty_box.clone())),
        (
            "screen-empty-rect",
            Some(empty_box),
            Some(valid_box.clone()),
        ),
        (
            "screen-invalid-rect",
            Some(Object::Integer(7)),
            Some(valid_box),
        ),
    ] {
        let mut stream = Stream::new(
            dictionary! {"Resources"=>dictionary! {}},
            b"0 0 1 rg 0 0 40 20 re f".to_vec(),
        );
        if let Some(rect) = rect {
            stream.dict.set("Rect", rect);
        }
        if let Some(bbox) = bbox {
            stream.dict.set("BBox", bbox);
        }
        let appearance = pdf.add_object(stream);
        let mut screen = annotation("Screen", name, page, 0);
        screen.set("AP", dictionary! {"N"=>appearance});
        annots.push(pdf.add_object(screen).into());
    }
    let annots = pdf.add_object(annots);
    pdf.get_dictionary_mut(page).unwrap().set("Annots", annots);
    let second_page = pdf.get_pages()[&2];
    let mut untouched = annotation("Screen", "second-page", second_page, 32);
    untouched.set("FS", file);
    let untouched = pdf.add_object(untouched);
    pdf.get_dictionary_mut(second_page)
        .unwrap()
        .set("Annots", vec![Object::Reference(untouched)]);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

pub fn annotations(pdf: &lopdf::Document, page: usize) -> Vec<(Option<ObjectId>, Dictionary)> {
    let page = pdf.get_pages()[&(page as u32 + 1)];
    pdf.get_dictionary(page)
        .unwrap()
        .get_deref(b"Annots", pdf)
        .ok()
        .and_then(|value| value.as_array().ok())
        .into_iter()
        .flatten()
        .map(|value| {
            (
                value.as_reference().ok(),
                pdf.dereference(value).unwrap().1.as_dict().unwrap().clone(),
            )
        })
        .collect()
}

pub fn assert_preserved(pdf: &lopdf::Document) {
    let annotations = annotations(pdf, 0);
    assert_eq!(
        annotations.len(),
        NAMES.len() + 2,
        "only widgets should be removed"
    );
    let find = |name: &str| {
        annotations
            .iter()
            .find(|(_, annotation)| {
                annotation.get(b"NM").and_then(Object::as_str).ok() == Some(name.as_bytes())
            })
            .unwrap_or_else(|| panic!("missing preserved annotation {name}"))
    };
    for name in NAMES {
        let (_, annotation) = find(name);
        assert_eq!(
            annotation.get(b"Contents").unwrap().as_str().unwrap(),
            name.as_bytes()
        );
        assert_eq!(
            annotation.get(b"P").unwrap().as_reference().unwrap(),
            pdf.get_pages()[&1]
        );
        let flags = match name {
            "popup" => 2,
            "reply" => 32,
            _ => 0,
        };
        assert_eq!(
            annotation.get(b"F").unwrap().as_i64().unwrap(),
            flags,
            "{name}"
        );
    }
    assert_eq!(
        annotations
            .iter()
            .filter(|(_, annotation)| {
                annotation.get(b"Subtype").unwrap().as_name().unwrap() == b"Link"
            })
            .count(),
        2
    );
    let (attachment_id, attachment) = find("attachment");
    let (popup_id, popup) = find("popup");
    let (reply_id, reply) = find("reply");
    assert_eq!(
        attachment.get(b"Popup").unwrap().as_reference().ok(),
        *popup_id
    );
    assert_eq!(
        popup.get(b"Parent").unwrap().as_reference().ok(),
        *attachment_id
    );
    assert_eq!(
        attachment.get(b"IRT").unwrap().as_reference().ok(),
        *reply_id
    );
    assert_eq!(
        reply.get(b"IRT").unwrap().as_reference().ok(),
        *attachment_id
    );
    let file = attachment.get(b"FS").unwrap().as_reference().unwrap();
    assert_eq!(reply.get(b"FS").unwrap().as_reference().unwrap(), file);
    let second_page = self::annotations(pdf, 1);
    assert_eq!(second_page.len(), 1);
    let second = &second_page[0].1;
    assert_eq!(second.get(b"NM").unwrap().as_str().unwrap(), b"second-page");
    assert_eq!(
        second.get(b"P").unwrap().as_reference().unwrap(),
        pdf.get_pages()[&2]
    );
    assert_eq!(second.get(b"F").unwrap().as_i64().unwrap(), 32);
    assert_eq!(second.get(b"FS").unwrap().as_reference().unwrap(), file);
    let file = pdf.get_dictionary(file).unwrap();
    assert_eq!(file.get(b"F").unwrap().as_str().unwrap(), b"regression.bin");
    let embedded = file
        .get_deref(b"EF", pdf)
        .unwrap()
        .as_dict()
        .unwrap()
        .get_deref(b"F", pdf)
        .unwrap()
        .as_stream()
        .unwrap();
    let content = if embedded.dict.get(b"Filter").is_ok() {
        embedded.decompressed_content().unwrap()
    } else {
        embedded.content.clone()
    };
    assert_eq!(content, PAYLOAD);
}
