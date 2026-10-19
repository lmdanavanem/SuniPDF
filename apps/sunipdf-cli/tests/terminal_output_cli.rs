use lopdf::{dictionary, Document, Object, Stream};
use std::process::Command;

#[test]
fn pdf_controls_are_filtered_in_text_and_escaped_in_json_without_modifying_the_pdf() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("controls.pdf");
    let mut pdf = Document::with_version("1.7");
    let cmap = pdf.add_object(Stream::new(dictionary! {}, b"/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def /CMapName /SafeTest def /CMapType 2 def 1 begincodespacerange <00> <FF> endcodespacerange 4 beginbfchar <41> <001B> <42> <009B> <4F> <004F> <4B> <004B> endbfchar endcmap CMapName currentdict /CMap defineresource pop end end".to_vec()));
    let font = pdf.add_object(dictionary! { "Type"=>"Font", "Subtype"=>"Type1", "BaseFont"=>"Helvetica", "Encoding"=>"WinAnsiEncoding", "ToUnicode"=>cmap });
    let pages = pdf.new_object_id();
    let content = pdf.add_object(Stream::new(
        dictionary! {},
        b"BT /F1 12 Tf 72 720 Td (ABOK) Tj ET".to_vec(),
    ));
    let page = pdf.add_object(dictionary! { "Type"=>"Page", "Parent"=>pages, "MediaBox"=>vec![0.into(),0.into(),612.into(),792.into()], "Resources"=>dictionary!{"Font"=>dictionary!{"F1"=>font}}, "Contents"=>content });
    pdf.objects.insert(
        pages,
        Object::Dictionary(dictionary! {"Type"=>"Pages", "Count"=>1, "Kids"=>vec![page.into()]}),
    );
    let catalog = pdf.add_object(dictionary! {"Type"=>"Catalog", "Pages"=>pages});
    pdf.trailer.set("Root", catalog);
    let title = "Title\u{9b} with control";
    let info = pdf.add_object(dictionary! {"Title"=>lopdf::text_string(title)});
    pdf.trailer.set("Info", info);
    pdf.save(&path).unwrap();
    let original = std::fs::read(&path).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"))
        .arg("text")
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("OK"));
    assert!(!text.contains(['\u{1b}', '\u{9b}']));

    let output = Command::new(env!("CARGO_BIN_EXE_sunipdf-cli"))
        .arg("info")
        .arg(&path)
        .output()
        .unwrap();
    assert!(output.status.success());
    let json = String::from_utf8(output.stdout).unwrap();
    assert!(json.contains("\\u009b") && !json.contains('\u{9b}'));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&json).unwrap()["title"],
        title
    );
    assert_eq!(std::fs::read(&path).unwrap(), original);
}
