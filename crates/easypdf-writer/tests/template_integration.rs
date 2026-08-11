//! Integration tests for easypdf-template using real PDF fixtures with form fields.
use easypdf_writer::PdfTemplateFiller;

fn make_form_pdf(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    let mut doc = lopdf::Document::new();

    let c = doc.add_object(lopdf::Object::Stream(lopdf::Stream::new(
        lopdf::Dictionary::new(),
        b" ".to_vec(),
    )));
    let mut f = lopdf::Dictionary::new();
    f.set("Type", lopdf::Object::Name(b"Annot".to_vec()));
    f.set("Subtype", lopdf::Object::Name(b"Widget".to_vec()));
    f.set("FT", lopdf::Object::Name(b"Tx".to_vec()));
    f.set(
        "T",
        lopdf::Object::String(b"name".to_vec(), lopdf::StringFormat::Literal),
    );
    f.set(
        "V",
        lopdf::Object::String(b"old".to_vec(), lopdf::StringFormat::Literal),
    );
    let fid = doc.add_object(lopdf::Object::Dictionary(f));

    let mut p = lopdf::Dictionary::new();
    p.set("Type", lopdf::Object::Name(b"Page".to_vec()));
    p.set(
        "MediaBox",
        lopdf::Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]),
    );
    p.set("Contents", lopdf::Object::Reference(c));
    p.set(
        "Annots",
        lopdf::Object::Array(vec![lopdf::Object::Reference(fid)]),
    );
    let pid = doc.add_object(lopdf::Object::Dictionary(p));

    let mut pages = lopdf::Dictionary::new();
    pages.set("Type", lopdf::Object::Name(b"Pages".to_vec()));
    pages.set(
        "Kids",
        lopdf::Object::Array(vec![lopdf::Object::Reference(pid)]),
    );
    pages.set("Count", lopdf::Object::Integer(1));
    let pages_id = doc.add_object(lopdf::Object::Dictionary(pages));

    let mut cat = lopdf::Dictionary::new();
    cat.set("Type", lopdf::Object::Name(b"Catalog".to_vec()));
    cat.set("Pages", lopdf::Object::Reference(pages_id));
    let cat_id = doc.add_object(lopdf::Object::Dictionary(cat));
    doc.trailer.set("Root", lopdf::Object::Reference(cat_id));
    doc.save(&path).unwrap();
    path
}

#[test]
fn test_fill_field_success_real() {
    let dir = std::env::temp_dir();
    let path = make_form_pdf(&dir, "tmpl_form.pdf");
    let mut filler = PdfTemplateFiller::open(&path).unwrap();
    let result = filler.fill_field("name", "Alice");
    assert!(result.is_ok());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_fill_fields_success_real() {
    let dir = std::env::temp_dir();
    let path = make_form_pdf(&dir, "tmpl_fields.pdf");
    let mut filler = PdfTemplateFiller::open(&path).unwrap();
    assert!(filler.fill_fields([("name", "Bob")]).is_ok());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_save_filled_real() {
    let dir = std::env::temp_dir();
    let path = make_form_pdf(&dir, "tmpl_save.pdf");
    let out = dir.join("tmpl_filled.pdf");
    let mut filler = PdfTemplateFiller::open(&path).unwrap();
    filler.fill_field("name", "Charlie").unwrap();
    filler.save(&out).unwrap();
    assert!(out.exists());
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn test_page_count_real() {
    let dir = std::env::temp_dir();
    let path = make_form_pdf(&dir, "tmpl_count.pdf");
    let filler = PdfTemplateFiller::open(&path).unwrap();
    assert_eq!(filler.page_count(), 1);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_into_inner_real() {
    let dir = std::env::temp_dir();
    let path = make_form_pdf(&dir, "tmpl_inner.pdf");
    let doc = PdfTemplateFiller::open(&path).unwrap().into_inner();
    assert_eq!(doc.get_pages().len(), 1);
    let _ = std::fs::remove_file(&path);
}
