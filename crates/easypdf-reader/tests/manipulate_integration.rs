//! Integration tests for easypdf-manipulate using real PDF fixtures.
use easypdf_core::Rotation;
use easypdf_reader::PdfManipulator;

fn make_fixture_pdf(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    // Reuse the same fixture creation logic
    let path = dir.join(name);
    let mut doc = lopdf::Document::new();

    let c = doc.add_object(lopdf::Object::Stream(lopdf::Stream::new(
        lopdf::Dictionary::new(),
        b"BT /F1 12 Tf 72 700 Td (Hello) Tj ET".to_vec(),
    )));
    let mut p = lopdf::Dictionary::new();
    p.set("Type", lopdf::Object::Name(b"Page".to_vec()));
    p.set(
        "MediaBox",
        lopdf::Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]),
    );
    p.set("Contents", lopdf::Object::Reference(c));
    let p1 = doc.add_object(lopdf::Object::Dictionary(p.clone()));
    let p2 = doc.add_object(lopdf::Object::Dictionary(p));

    let mut pages = lopdf::Dictionary::new();
    pages.set("Type", lopdf::Object::Name(b"Pages".to_vec()));
    pages.set(
        "Kids",
        lopdf::Object::Array(vec![
            lopdf::Object::Reference(p1),
            lopdf::Object::Reference(p2),
        ]),
    );
    pages.set("Count", lopdf::Object::Integer(2));
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
fn test_merge_two_real_pdfs() {
    let dir = std::env::temp_dir();
    let p1 = make_fixture_pdf(&dir, "manip_a.pdf");
    let p2 = make_fixture_pdf(&dir, "manip_b.pdf");
    let out = dir.join("manip_merged.pdf");
    let result = PdfManipulator::merge_files(&[&p1, &p2], &out);
    assert!(result.is_ok());
    assert!(out.exists());
    let merged = lopdf::Document::load(&out).unwrap();
    assert_eq!(merged.get_pages().len(), 4);
    let _ = std::fs::remove_file(&p1);
    let _ = std::fs::remove_file(&p2);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn test_extract_pages_real() {
    let dir = std::env::temp_dir();
    let p = make_fixture_pdf(&dir, "manip_extract.pdf");
    let m = PdfManipulator::open(&p).unwrap();
    let count = m.page_count();
    eprintln!("page_count={count}");
    let mut document = m.extract_pages(0..1).unwrap();
    assert_eq!(document.get_pages().len(), 1);
    let output = dir.join("manip_extracted.pdf");
    document.save(&output).unwrap();
    let reopened = lopdf::Document::load(&output).unwrap();
    assert_eq!(reopened.get_pages().len(), 1);
    let _ = std::fs::remove_file(&p);
    let _ = std::fs::remove_file(&output);
}

#[test]
fn test_rotate_reorder_real() {
    let dir = std::env::temp_dir();
    let p = make_fixture_pdf(&dir, "manip_rotate.pdf");
    let mut m = PdfManipulator::open(&p).unwrap();
    assert!(m.rotate_page(1, Rotation::Clockwise90).is_ok());
    assert!(m.reorder_pages(&[1, 0]).is_ok());
    let out = dir.join("manip_rotated.pdf");
    m.save(&out).unwrap();
    assert!(out.exists());
    let _ = std::fs::remove_file(&p);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn test_page_count_real() {
    let dir = std::env::temp_dir();
    let p = make_fixture_pdf(&dir, "manip_count.pdf");
    let m = PdfManipulator::open(&p).unwrap();
    assert_eq!(m.page_count(), 2);
    let _ = std::fs::remove_file(&p);
}

#[test]
fn test_into_inner_real() {
    let dir = std::env::temp_dir();
    let p = make_fixture_pdf(&dir, "manip_inner.pdf");
    let doc = PdfManipulator::open(&p).unwrap().into_inner();
    assert_eq!(doc.get_pages().len(), 2);
    let _ = std::fs::remove_file(&p);
}
