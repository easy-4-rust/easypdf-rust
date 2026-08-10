//! Shared PDF fixture generation for integration testing.
//!
//! Creates valid, multi-page PDFs with embedded fonts, form fields,
//! and extractable text content to exercise all code paths.

use std::path::Path;

/// Create a valid multi-page PDF with embedded font, form field, and extractable text.
/// Returns the path to the created file.
pub fn make_fixture_pdf(dir: &Path, name: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    let mut doc = lopdf::Document::new();

    // --- Page 1: text content ---
    let content1_id = doc.add_object(lopdf::Object::Stream(lopdf::Stream::new(
        lopdf::Dictionary::new(),
        b"BT /F1 12 Tf 72 700 Td (Hello World) Tj ET".to_vec(),
    )));

    // Embedded font with /FontFile2 entry
    let mut font_dict = lopdf::Dictionary::new();
    font_dict.set("Type", lopdf::Object::Name(b"Font".to_vec()));
    font_dict.set("Subtype", lopdf::Object::Name(b"Type1".to_vec()));
    font_dict.set("BaseFont", lopdf::Object::Name(b"Helvetica".to_vec()));
    font_dict.set(
        "FontFile2",
        lopdf::Object::Stream(lopdf::Stream::new(
            lopdf::Dictionary::new(),
            vec![0u8; 100], // placeholder font data
        )),
    );
    let font_id = doc.add_object(lopdf::Object::Dictionary(font_dict));

    let mut resources1 = lopdf::Dictionary::new();
    let mut fonts1 = lopdf::Dictionary::new();
    fonts1.set("F1", lopdf::Object::Reference(font_id));
    resources1.set("Font", lopdf::Object::Dictionary(fonts1));
    let res1_id = doc.add_object(lopdf::Object::Dictionary(resources1.clone()));

    let mut page1 = lopdf::Dictionary::new();
    page1.set("Type", lopdf::Object::Name(b"Page".to_vec()));
    page1.set(
        "MediaBox",
        lopdf::Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]),
    );
    page1.set("Contents", lopdf::Object::Reference(content1_id));
    page1.set("Resources", lopdf::Object::Reference(res1_id));
    let page1_id = doc.add_object(lopdf::Object::Dictionary(page1));

    // --- Page 2: form field ---
    let content2_id = doc.add_object(lopdf::Object::Stream(lopdf::Stream::new(
        lopdf::Dictionary::new(),
        b"BT /F1 10 Tf 72 700 Td (Page Two) Tj ET".to_vec(),
    )));

    // Form field widget annotation
    let mut field = lopdf::Dictionary::new();
    field.set("Type", lopdf::Object::Name(b"Annot".to_vec()));
    field.set("Subtype", lopdf::Object::Name(b"Widget".to_vec()));
    field.set("FT", lopdf::Object::Name(b"Tx".to_vec()));
    field.set(
        "T",
        lopdf::Object::String(b"customer_name".to_vec(), lopdf::StringFormat::Literal),
    );
    field.set(
        "V",
        lopdf::Object::String(b"".to_vec(), lopdf::StringFormat::Literal),
    );
    let field_id = doc.add_object(lopdf::Object::Dictionary(field));

    let mut page2 = lopdf::Dictionary::new();
    page2.set("Type", lopdf::Object::Name(b"Page".to_vec()));
    page2.set(
        "MediaBox",
        lopdf::Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]),
    );
    page2.set("Contents", lopdf::Object::Reference(content2_id));
    page2.set(
        "Annots",
        lopdf::Object::Array(vec![lopdf::Object::Reference(field_id)]),
    );
    let res2_id = doc.add_object(lopdf::Object::Dictionary(resources1.clone()));
    page2.set("Resources", lopdf::Object::Reference(res2_id));
    let page2_id = doc.add_object(lopdf::Object::Dictionary(page2));

    // Pages tree
    let mut pages_dict = lopdf::Dictionary::new();
    pages_dict.set("Type", lopdf::Object::Name(b"Pages".to_vec()));
    pages_dict.set(
        "Kids",
        lopdf::Object::Array(vec![
            lopdf::Object::Reference(page1_id),
            lopdf::Object::Reference(page2_id),
        ]),
    );
    pages_dict.set("Count", lopdf::Object::Integer(2));
    let pages_id = doc.add_object(lopdf::Object::Dictionary(pages_dict));

    // Catalog
    let mut catalog = lopdf::Dictionary::new();
    catalog.set("Type", lopdf::Object::Name(b"Catalog".to_vec()));
    catalog.set("Pages", lopdf::Object::Reference(pages_id));
    let catalog_id = doc.add_object(lopdf::Object::Dictionary(catalog));
    doc.trailer
        .set("Root", lopdf::Object::Reference(catalog_id));

    doc.save(&path).unwrap();
    path
}
