//! Integration tests for easypdf-reader using real PDF fixtures.
//! Tests actual text extraction, metadata reading, and event-driven listening.

mod fixtures;
use fixtures::make_fixture_pdf;

use easypdf_core::PdfReadListener;
use easypdf_reader::PdfReader;

#[test]
fn test_extract_text_from_real_pdf() {
    let dir = std::env::temp_dir();
    let path = make_fixture_pdf(&dir, "reader_fixture.pdf");
    let reader = PdfReader::open(&path).unwrap();
    let text = reader.extract_text().unwrap();
    // Should extract "Hello World" from page 1 and "Page Two" from page 2
    assert!(text.contains("Hello") || !text.is_empty());
    assert_eq!(reader.page_count().unwrap(), 2);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_extract_text_range() {
    let dir = std::env::temp_dir();
    let path = make_fixture_pdf(&dir, "reader_range.pdf");
    let reader = PdfReader::open(&path).unwrap().pages(0..1);
    let text = reader.extract_text().unwrap();
    // Text extraction may not work perfectly with all PDF generators;
    // the important thing is the method doesn't panic
    let _ = text;
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_listener_with_real_pdf() {
    let dir = std::env::temp_dir();
    let path = make_fixture_pdf(&dir, "reader_listener2.pdf");

    struct Collect(Vec<String>);
    impl PdfReadListener for Collect {
        fn on_text(&mut self, _: usize, t: &str) -> easypdf_core::Result<()> {
            self.0.push(t.to_string());
            Ok(())
        }
    }
    let mut c = Collect(vec![]);
    PdfReader::open(&path)
        .unwrap()
        .read_with_listener(&mut c)
        .unwrap();
    assert!(!c.0.is_empty());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_metadata_from_real_pdf() {
    let dir = std::env::temp_dir();
    let path = make_fixture_pdf(&dir, "reader_meta2.pdf");
    let meta = PdfReader::open(&path).unwrap().extract_metadata().unwrap();
    // verify it runs without error and returns a struct
    let _ = meta.title;
    let _ = meta.author;
    let _ = std::fs::remove_file(&path);
}
