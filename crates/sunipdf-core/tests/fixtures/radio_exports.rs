use lopdf::{Document, Object, ObjectId};

pub const FIELD_NAME: &str = "paiement";
pub const EXPORTS: [&str; 3] = ["Card with space", "Virement \u{e9} \u{96ea}", "Cash"];

#[track_caller]
pub fn field_id(pdf: &Document) -> ObjectId {
    pdf.objects
        .iter()
        .find_map(|(&id, object)| {
            let dictionary = object.as_dict().ok()?;
            (lopdf::decode_text_string(pdf.dereference(dictionary.get(b"T").ok()?).ok()?.1)
                .ok()?
                .as_str()
                == FIELD_NAME)
                .then_some(id)
        })
        .unwrap_or_else(|| {
            let names: Vec<_> = pdf
                .objects
                .iter()
                .filter_map(|(&id, object)| {
                    let name = object.as_dict().ok()?.get(b"T").ok()?;
                    let resolved = pdf.dereference(name).ok()?.1;
                    Some((id, lopdf::decode_text_string(resolved), resolved))
                })
                .collect();
            panic!(
                "missing radio field {FIELD_NAME:?}; encrypted={}; names={names:?}",
                pdf.is_encrypted()
            );
        })
}

pub fn source(invisible_first: bool) -> Vec<u8> {
    let mut pdf = Document::load_mem(include_bytes!("acroform.pdf")).unwrap();
    let field = field_id(&pdf);
    let unicode = pdf.add_object(lopdf::text_string(EXPORTS[1]));
    let dictionary = pdf.get_dictionary_mut(field).unwrap();
    dictionary.set(
        "Opt",
        vec![
            lopdf::text_string(EXPORTS[0]),
            Object::Reference(unicode),
            lopdf::text_string(EXPORTS[2]),
        ],
    );
    dictionary.set("V", Object::Name(b"1".to_vec()));
    // Allow clearing so the same fixture checks an empty selection and import restoration.
    dictionary.set("Ff", 1 << 15);
    let first = dictionary.get(b"Kids").unwrap().as_array().unwrap()[0]
        .as_reference()
        .unwrap();
    if invisible_first {
        for page in pdf.get_pages().values() {
            let annots = pdf
                .get_dictionary_mut(*page)
                .unwrap()
                .get_mut(b"Annots")
                .unwrap()
                .as_array_mut()
                .unwrap();
            annots.retain(|object| object.as_reference().ok() != Some(first));
        }
    }
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}
