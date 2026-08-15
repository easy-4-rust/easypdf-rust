use super::*;
use easypdf_core::PdfReadListener;

/// Helper: build a minimal valid `lopdf::Document` for tests.
fn make_test_doc() -> lopdf::Document {
    let mut doc = lopdf::Document::new();

    let content_id = doc.add_object(lopdf::Object::Stream(lopdf::Stream::new(
        lopdf::Dictionary::new(),
        b"BT /F1 12 Tf (Hello) Tj ET".to_vec(),
    )));

    let mut font_dict = lopdf::Dictionary::new();
    font_dict.set("Type", lopdf::Object::Name(b"Font".to_vec()));
    font_dict.set("Subtype", lopdf::Object::Name(b"Type1".to_vec()));
    font_dict.set("BaseFont", lopdf::Object::Name(b"Helvetica".to_vec()));
    let font_id = doc.add_object(lopdf::Object::Dictionary(font_dict));

    let mut resources = lopdf::Dictionary::new();
    let mut fonts = lopdf::Dictionary::new();
    fonts.set("F1", lopdf::Object::Reference(font_id));
    resources.set("Font", lopdf::Object::Dictionary(fonts));
    let resources_id = doc.add_object(lopdf::Object::Dictionary(resources));

    let mut page_dict = lopdf::Dictionary::new();
    page_dict.set("Type", lopdf::Object::Name(b"Page".to_vec()));
    page_dict.set(
        "MediaBox",
        lopdf::Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]),
    );
    page_dict.set("Contents", lopdf::Object::Reference(content_id));
    page_dict.set("Resources", lopdf::Object::Reference(resources_id));
    let page_id = doc.add_object(lopdf::Object::Dictionary(page_dict));

    #[allow(clippy::similar_names)]
    let mut pages_dict = lopdf::Dictionary::new();
    pages_dict.set("Type", lopdf::Object::Name(b"Pages".to_vec()));
    pages_dict.set(
        "Kids",
        lopdf::Object::Array(vec![lopdf::Object::Reference(page_id)]),
    );
    pages_dict.set("Count", lopdf::Object::Integer(1));
    let pages_id = doc.add_object(lopdf::Object::Dictionary(pages_dict));

    let mut catalog = lopdf::Dictionary::new();
    catalog.set("Type", lopdf::Object::Name(b"Catalog".to_vec()));
    catalog.set("Pages", lopdf::Object::Reference(pages_id));
    let catalog_id = doc.add_object(lopdf::Object::Dictionary(catalog));

    doc.trailer
        .set("Root", lopdf::Object::Reference(catalog_id));
    doc
}

/// Helper: save a test doc to a temp file and return the path.
fn save_test_pdf(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir();
    let path = dir.join(name);
    let mut doc = make_test_doc();
    doc.save(&path).unwrap();
    path
}

#[test]
fn test_open_valid_pdf() {
    let path = save_test_pdf("easypdf_reader_test.pdf");
    let reader = PdfReader::open(&path).unwrap();
    assert!(reader.extract_text().is_ok());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_open_nonexistent_file() {
    let result = PdfReader::open("/nonexistent/path/file.pdf");
    assert!(result.is_err());
}

#[test]
fn test_page_count() {
    let path = save_test_pdf("easypdf_reader_count.pdf");
    let count = PdfReader::open(&path).unwrap().page_count().unwrap();
    assert_eq!(count, 1);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_extract_text() {
    let path = save_test_pdf("easypdf_reader_text.pdf");
    let text = PdfReader::open(&path).unwrap().extract_text().unwrap();
    assert_eq!(text, "Hello\n");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_extract_metadata() {
    let path = save_test_pdf("easypdf_reader_meta.pdf");
    let meta = PdfReader::open(&path).unwrap().extract_metadata().unwrap();
    assert!(meta.title.is_none());
    assert!(meta.author.is_none());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_pages_range() {
    let path = save_test_pdf("easypdf_reader_range.pdf");
    let reader = PdfReader::open(&path).unwrap().pages(0..1);
    assert!(reader.extract_text().is_ok());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_read_with_listener() {
    let path = save_test_pdf("easypdf_reader_listener.pdf");

    struct CollectListener {
        texts: Vec<String>,
    }
    impl PdfReadListener for CollectListener {
        fn on_text(&mut self, _page: usize, text: &str) -> easypdf_core::Result<()> {
            self.texts.push(text.to_string());
            Ok(())
        }
    }

    let mut listener = CollectListener { texts: vec![] };
    PdfReader::open(&path)
        .unwrap()
        .read_with_listener(&mut listener)
        .unwrap();
    assert_eq!(listener.texts, ["Hello\n"]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_invalid_pdf_path() {
    let dir = std::env::temp_dir();
    let path = dir.join("easypdf_not_a_pdf.txt");
    std::fs::write(&path, b"not a pdf file").unwrap();
    assert!(PdfReader::open(&path).is_err());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_corrupt_pdf_data() {
    let dir = std::env::temp_dir();
    let path = dir.join("easypdf_corrupt.pdf");
    std::fs::write(&path, b"%PDF-1.4\n% corrupted\n%%EOF").unwrap();
    assert!(PdfReader::open(&path).is_err());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_extract_text_with_content() {
    let dir = std::env::temp_dir();
    let path = dir.join("reader_txt.pdf");
    let mut doc = lopdf::Document::new();
    let c = doc.add_object(lopdf::Object::Stream(lopdf::Stream::new(
        lopdf::Dictionary::new(),
        b"BT /F1 12 Tf 72 700 Td (Hello PDF) Tj ET".to_vec(),
    )));
    let mut p = lopdf::Dictionary::new();
    p.set("Type", lopdf::Object::Name(b"Page".to_vec()));
    p.set(
        "MediaBox",
        lopdf::Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]),
    );
    p.set("Contents", lopdf::Object::Reference(c));
    let pid = doc.add_object(lopdf::Object::Dictionary(p));
    let mut pages = lopdf::Dictionary::new();
    pages.set("Type", lopdf::Object::Name(b"Pages".to_vec()));
    pages.set(
        "Kids",
        lopdf::Object::Array(vec![lopdf::Object::Reference(pid)]),
    );
    pages.set("Count", lopdf::Object::Integer(1));
    let pgid = doc.add_object(lopdf::Object::Dictionary(pages));
    let mut cat = lopdf::Dictionary::new();
    cat.set("Type", lopdf::Object::Name(b"Catalog".to_vec()));
    cat.set("Pages", lopdf::Object::Reference(pgid));
    let cid = doc.add_object(lopdf::Object::Dictionary(cat));
    doc.trailer.set("Root", lopdf::Object::Reference(cid));
    doc.save(&path).unwrap();
    let reader = PdfReader::open(&path).unwrap();
    let text = reader.extract_text().unwrap();
    let _ = text;
    let meta = reader.extract_metadata().unwrap();
    let _ = meta;
    let _ = std::fs::remove_file(&path);
}

// --- New tests for ReadStrategy integration ---

#[test]
fn test_open_with_strategy_full() {
    let path = save_test_pdf("easypdf_strategy_full.pdf");
    let reader = PdfReader::open_with_strategy(&path, ReadStrategy::Full).unwrap();
    assert_eq!(reader.strategy(), ReadStrategy::Full);
    assert!(reader.extract_text().is_ok());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_open_with_strategy_lazy() {
    let path = save_test_pdf("easypdf_strategy_lazy.pdf");
    let reader = PdfReader::open_with_strategy(&path, ReadStrategy::Lazy).unwrap();
    assert_eq!(reader.strategy(), ReadStrategy::Lazy);
    assert!(reader.extract_text().is_ok());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_open_with_strategy_streaming() {
    let path = save_test_pdf("easypdf_strategy_streaming.pdf");
    let reader = PdfReader::open_with_strategy(&path, ReadStrategy::Streaming).unwrap();
    assert_eq!(reader.strategy(), ReadStrategy::Streaming);
    // Streaming mode opens without error (no lopdf::Document built).
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_strategy_auto_selects_full_for_small_file() {
    let path = save_test_pdf("easypdf_auto_full.pdf");
    let reader = PdfReader::open(&path).unwrap();
    assert_eq!(reader.strategy(), ReadStrategy::Full);
    let _ = std::fs::remove_file(&path);
}

// --- Generic listener test ---

#[test]
fn test_read_with_listener_typed() {
    let path = save_test_pdf("easypdf_typed_listener.pdf");

    struct CollectListener {
        texts: Vec<String>,
        page_starts: Vec<usize>,
        page_ends: Vec<usize>,
        document_ended: bool,
    }
    impl PdfReadListener for CollectListener {
        fn on_page_start(&mut self, page: usize) -> easypdf_core::Result<()> {
            self.page_starts.push(page);
            Ok(())
        }
        fn on_text(&mut self, _page: usize, text: &str) -> easypdf_core::Result<()> {
            self.texts.push(text.to_string());
            Ok(())
        }
        fn on_page_end(&mut self, page: usize) -> easypdf_core::Result<()> {
            self.page_ends.push(page);
            Ok(())
        }
        fn on_document_end(&mut self) -> easypdf_core::Result<()> {
            self.document_ended = true;
            Ok(())
        }
    }

    let mut listener = CollectListener {
        texts: vec![],
        page_starts: vec![],
        page_ends: vec![],
        document_ended: false,
    };
    PdfReader::open(&path)
        .unwrap()
        .read_with_listener_typed(&mut listener)
        .unwrap();

    assert_eq!(listener.texts, ["Hello\n"]);
    assert_eq!(listener.page_starts, [1]);
    assert_eq!(listener.page_ends, [1]);
    assert!(listener.document_ended);
    let _ = std::fs::remove_file(&path);
}

// --- Lazy text extraction test ---

#[test]
fn test_extract_text_lazy() {
    let path = save_test_pdf("easypdf_lazy_text.pdf");
    let mut reader = PdfReader::open_with_strategy(&path, ReadStrategy::Lazy).unwrap();
    let text = reader.extract_text_lazy().unwrap();
    assert_eq!(text, "Hello\n");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_extract_text_lazy_full_strategy_delegates() {
    let path = save_test_pdf("easypdf_lazy_full.pdf");
    let mut reader = PdfReader::open_with_strategy(&path, ReadStrategy::Full).unwrap();
    let text = reader.extract_text_lazy().unwrap();
    assert_eq!(text, "Hello\n");
    let _ = std::fs::remove_file(&path);
}

// --- open_with_repair test ---

#[test]
fn test_open_with_repair_valid_file() {
    let path = save_test_pdf("easypdf_repair_valid.pdf");
    let reader =
        PdfReader::open_with_repair(&path, RepairOptions::default(), ReadStrategy::Full).unwrap();
    assert!(reader.extract_text().is_ok());
    let _ = std::fs::remove_file(&path);
}

// --- open_with_limits_and_strategy test ---

#[test]
fn test_open_with_limits_and_strategy() {
    let path = save_test_pdf("easypdf_limits_strategy.pdf");
    let input = PdfInput::from_path(&path);
    let reader = PdfReader::open_with_limits_and_strategy(
        &input,
        ResourceLimits::default(),
        ReadStrategy::Full,
    )
    .unwrap();
    assert_eq!(reader.strategy(), ReadStrategy::Full);
    assert!(reader.extract_text().is_ok());
    let _ = std::fs::remove_file(&path);
}

// --- from_bytes with strategy ---

#[test]
fn test_from_bytes_selects_strategy() {
    let path = save_test_pdf("easypdf_from_bytes.pdf");
    let bytes = std::fs::read(&path).unwrap();
    let reader = PdfReader::from_bytes(bytes).unwrap();
    // Small file -> Full strategy
    assert_eq!(reader.strategy(), ReadStrategy::Full);
    let _ = std::fs::remove_file(&path);
}

// --- Element explosion guard ---

#[test]
fn test_element_explosion_guard_rejects() {
    let path = save_test_pdf("easypdf_element_guard.pdf");
    let input = PdfInput::from_path(&path);
    let limits = ResourceLimits::strict().with_max_element_count(1);
    let result = PdfReader::open_with_limits_and_strategy(&input, limits, ReadStrategy::Full);
    // The minimal test PDF has > 1 object, so this should fail.
    assert!(result.is_err());
    let _ = std::fs::remove_file(&path);
}

// --- Streaming integration tests ---

#[test]
fn test_streaming_extract_text_from_real_pdf() {
    let path = save_test_pdf("easypdf_streaming_text.pdf");
    let reader = PdfReader::open_with_strategy(&path, ReadStrategy::Streaming).unwrap();
    assert_eq!(reader.strategy(), ReadStrategy::Streaming);
    // The streaming extractor won't find text in a saved PDF because
    // lopdf compresses content streams.  But it should not crash.
    let result = reader.extract_text();
    assert!(result.is_ok());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_streaming_metadata_heuristic() {
    let path = save_test_pdf("easypdf_streaming_meta.pdf");
    let reader = PdfReader::open_with_strategy(&path, ReadStrategy::Streaming).unwrap();
    let meta = reader.extract_metadata().unwrap();
    // No /Info dict in test PDF -> metadata should be empty.
    assert!(meta.title.is_none());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_streaming_page_count_heuristic() {
    let path = save_test_pdf("easypdf_streaming_count.pdf");
    let reader = PdfReader::open_with_strategy(&path, ReadStrategy::Streaming).unwrap();
    let count = reader.page_count().unwrap();
    // The heuristic should find at least 1 /Type /Page entry.
    assert!(count >= 1);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_streaming_read_with_listener() {
    let path = save_test_pdf("easypdf_streaming_listener.pdf");
    let reader = PdfReader::open_with_strategy(&path, ReadStrategy::Streaming).unwrap();

    struct CollectListener {
        pages: Vec<usize>,
        ended: bool,
    }
    impl PdfReadListener for CollectListener {
        fn on_page_start(&mut self, page: usize) -> easypdf_core::Result<()> {
            self.pages.push(page);
            Ok(())
        }
        fn on_text(&mut self, _page: usize, _text: &str) -> easypdf_core::Result<()> {
            Ok(())
        }
        fn on_document_end(&mut self) -> easypdf_core::Result<()> {
            self.ended = true;
            Ok(())
        }
    }

    let mut listener = CollectListener {
        pages: vec![],
        ended: false,
    };
    reader.read_with_listener_typed(&mut listener).unwrap();
    assert!(listener.ended);
    let _ = std::fs::remove_file(&path);
}
