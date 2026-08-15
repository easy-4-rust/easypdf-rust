use easypdf_core::{PdfReadListener, ResourceLimits};

use super::super::scanner;
use super::super::scanner::{StreamRange, StreamScanner};

// --- Stream boundary detection ---

#[test]
fn find_streams_basic() {
    let data = b"some header\nstream\nHello World\nendstream\ntrailer";
    let streams = scanner::find_streams_for_test(data);
    assert_eq!(streams.len(), 1);
    assert_eq!(
        &data[streams[0].data_start..streams[0].data_end],
        b"Hello World"
    );
}

#[test]
fn find_streams_crlf() {
    let data = b"stream\r\nData here\r\nendstream";
    let streams = scanner::find_streams_for_test(data);
    assert_eq!(streams.len(), 1);
    assert_eq!(
        &data[streams[0].data_start..streams[0].data_end],
        b"Data here"
    );
}

#[test]
fn find_streams_multiple() {
    let data = b"stream\nAAA\nendstream\nfoo\nstream\nBBB\nendstream";
    let streams = scanner::find_streams_for_test(data);
    assert_eq!(streams.len(), 2);
    assert_eq!(&data[streams[0].data_start..streams[0].data_end], b"AAA");
    assert_eq!(&data[streams[1].data_start..streams[1].data_end], b"BBB");
}

#[test]
fn find_streams_empty_data() {
    let data = b"stream\n\nendstream";
    let streams = scanner::find_streams_for_test(data);
    // Empty stream (data between stream\n and endstream is just \n which
    // gets trimmed to empty).
    assert_eq!(streams.len(), 0);
}

#[test]
fn find_streams_no_streams() {
    let data = b"%PDF-1.4 no streams here";
    let streams = scanner::find_streams_for_test(data);
    assert!(streams.is_empty());
}

#[test]
fn find_streams_with_trailing_eol() {
    let data = b"stream\nHello\nendstream";
    let streams = scanner::find_streams_for_test(data);
    assert_eq!(streams.len(), 1);
    // The trailing EOL before endstream is trimmed.
    assert_eq!(&data[streams[0].data_start..streams[0].data_end], b"Hello");
}

// --- FlateDecode detection ---

#[test]
fn flatedecode_detection() {
    let mut data = Vec::new();
    // Simulate a PDF dict + stream.
    data.extend_from_slice(b"<< /Length 10 /Filter /FlateDecode >>\nstream\n");
    let stream_start = data.len();
    data.extend_from_slice(b"compressed");
    let stream_end = data.len();
    data.extend_from_slice(b"\nendstream");

    let scanner = StreamScanner::new(&data, ResourceLimits::default());
    let range = StreamRange {
        data_start: stream_start,
        data_end: stream_end,
    };
    assert!(scanner.has_flatedecode_filter(&range));
}

#[test]
fn no_flatedecode_filter() {
    let mut data = Vec::new();
    data.extend_from_slice(b"<< /Length 10 >>\nstream\n");
    let stream_start = data.len();
    data.extend_from_slice(b"plaintext");
    let stream_end = data.len();
    data.extend_from_slice(b"\nendstream");

    let scanner = StreamScanner::new(&data, ResourceLimits::default());
    let range = StreamRange {
        data_start: stream_start,
        data_end: stream_end,
    };
    assert!(!scanner.has_flatedecode_filter(&range));
}

// --- Decompression ---

#[test]
fn decompress_valid_zlib() {
    use flate2::write::ZlibEncoder;
    use std::io::Write;

    let original = b"BT /F1 12 Tf (Compressed) Tj ET";
    let mut encoder = ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(original).unwrap();
    let compressed = encoder.finish().unwrap();

    let scanner = StreamScanner::new(b"", ResourceLimits::default());
    let decompressed = scanner.decompress_stream(&compressed).unwrap();
    assert_eq!(&decompressed, original);
}

#[test]
fn decompress_invalid_data() {
    let scanner = StreamScanner::new(b"", ResourceLimits::default());
    let result = scanner.decompress_stream(b"not valid zlib data");
    assert!(result.is_err());
}

// --- Full scan with listener ---

#[test]
fn scan_plaintext_stream() {
    struct Collector {
        texts: Vec<String>,
        pages: Vec<usize>,
    }
    impl PdfReadListener for Collector {
        fn on_page_start(&mut self, page: usize) -> easypdf_core::Result<()> {
            self.pages.push(page);
            Ok(())
        }
        fn on_text(&mut self, _page: usize, text: &str) -> easypdf_core::Result<()> {
            self.texts.push(text.to_string());
            Ok(())
        }
    }

    let mut data = Vec::new();
    data.extend_from_slice(b"<< /Length 34 >>\nstream\n");
    data.extend_from_slice(b"BT /F1 12 Tf (Hello Stream) Tj ET\n");
    data.extend_from_slice(b"endstream\n");

    let mut collector = Collector {
        texts: vec![],
        pages: vec![],
    };
    let scanner = StreamScanner::new(&data, ResourceLimits::default());
    let result = scanner.scan(&mut collector).unwrap();

    assert!(result.text_extracted);
    assert_eq!(result.streams_processed, 1);
    assert_eq!(result.pages_scanned, 1);
    assert!(!collector.texts.is_empty());
    assert!(collector.texts[0].contains("Hello Stream"));
}

// --- Page counting ---

#[test]
fn page_count_basic() {
    let data = b"<< /Type /Page /Contents 1 0 R >>\n<< /Type /Page /Contents 2 0 R >>";
    let scanner = StreamScanner::new(data, ResourceLimits::default());
    assert_eq!(scanner.page_count(), 2);
}

#[test]
fn page_count_excludes_pages_type() {
    let data = b"<< /Type /Pages /Count 5 >>\n<< /Type /Page >>";
    let scanner = StreamScanner::new(data, ResourceLimits::default());
    assert_eq!(scanner.page_count(), 1);
}

// --- Metadata ---

#[test]
fn metadata_quick_from_bytes() {
    let data = b"<< /Title (My Document) /Author (Test Author) >>";
    let scanner = StreamScanner::new(data, ResourceLimits::default());
    let meta = scanner.extract_metadata_quick();
    assert_eq!(meta.title.as_deref(), Some("My Document"));
    assert_eq!(meta.author.as_deref(), Some("Test Author"));
}

#[test]
fn metadata_quick_empty() {
    let data = b"<< /Length 0 >>";
    let scanner = StreamScanner::new(data, ResourceLimits::default());
    let meta = scanner.extract_metadata_quick();
    assert!(meta.title.is_none());
    assert!(meta.author.is_none());
}

// --- Decompression bomb guard ---

#[test]
fn decompression_bomb_rejected() {
    use flate2::write::ZlibEncoder;
    use std::io::Write;

    // Create a small compressed payload that decompresses to something
    // exceeding strict limits.
    let original = vec![b'A'; 100_000];
    let mut encoder = ZlibEncoder::new(Vec::new(), flate2::Compression::best());
    encoder.write_all(&original).unwrap();
    let compressed = encoder.finish().unwrap();

    // Use a very strict limit.
    let limits = ResourceLimits::strict().with_max_decompressed_size(1000);
    let scanner = StreamScanner::new(b"", limits);
    let result = scanner.decompress_stream(&compressed);
    assert!(result.is_err());
}

// --- Integration: scan a complete mini-PDF byte stream ---

#[test]
fn scan_mini_pdf_stream() {
    // Build a minimal PDF-like byte stream (not a valid PDF, but exercises
    // the full scan pipeline).  Two uncompressed streams with text content.
    let mut data = Vec::new();
    data.extend_from_slice(b"<< /Length 40 >>\nstream\n");
    data.extend_from_slice(b"BT /F1 12 Tf 72 700 Td (First) Tj ET\n");
    data.extend_from_slice(b"endstream\n");

    data.extend_from_slice(b"<< /Length 41 >>\nstream\n");
    data.extend_from_slice(b"BT /F1 12 Tf 72 600 Td (Second) Tj ET\n");
    data.extend_from_slice(b"endstream\n");

    struct Collector {
        texts: Vec<String>,
    }
    impl PdfReadListener for Collector {
        fn on_text(&mut self, _page: usize, text: &str) -> easypdf_core::Result<()> {
            self.texts.push(text.to_string());
            Ok(())
        }
    }

    let mut collector = Collector { texts: vec![] };
    let scanner = StreamScanner::new(&data, ResourceLimits::default());
    let result = scanner.scan(&mut collector).unwrap();

    assert_eq!(result.streams_processed, 2);
    assert!(result.text_extracted);
    assert_eq!(collector.texts.len(), 2);
    assert!(collector.texts[0].contains("First"));
    assert!(collector.texts[1].contains("Second"));
}

#[test]
fn scan_flatedecode_stream() {
    use flate2::write::ZlibEncoder;
    use std::io::Write;

    // Use content whose zlib-compressed form is known to not contain
    // `\nendstream` as a byte subsequence.
    let content = b"BT /F1 12 Tf (Compressed) Tj ET";
    let mut encoder = ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(content).unwrap();
    let compressed = encoder.finish().unwrap();

    // Verify the compressed data does not contain `\nendstream`.
    let needle = b"\nendstream";
    assert!(
        !compressed.windows(needle.len()).any(|w| w == needle),
        "compressed data contains \\nendstream -- test is non-deterministic"
    );

    let mut data = Vec::new();
    data.extend_from_slice(
        format!(
            "<< /Length {} /Filter /FlateDecode >>\nstream\n",
            compressed.len()
        )
        .as_bytes(),
    );
    data.extend_from_slice(&compressed);
    data.extend_from_slice(b"\nendstream\n");

    struct Collector {
        texts: Vec<String>,
    }
    impl PdfReadListener for Collector {
        fn on_text(&mut self, _page: usize, text: &str) -> easypdf_core::Result<()> {
            self.texts.push(text.to_string());
            Ok(())
        }
    }

    let mut collector = Collector { texts: vec![] };
    let scanner = StreamScanner::new(&data, ResourceLimits::default());
    let result = scanner.scan(&mut collector).unwrap();

    assert_eq!(result.streams_processed, 1);
    assert!(result.text_extracted);
    assert_eq!(collector.texts.len(), 1);
    assert!(collector.texts[0].contains("Compressed"));
}

// --- Resource limit enforcement ---

#[test]
fn scan_text_limit_exceeded() {
    struct NoopListener;
    impl PdfReadListener for NoopListener {
        fn on_text(&mut self, _page: usize, _text: &str) -> easypdf_core::Result<()> {
            Ok(())
        }
    }

    let mut data = Vec::new();
    data.extend_from_slice(b"<< /Length 20 >>\nstream\n");
    data.extend_from_slice(b"BT (LongText) Tj ET\n");
    data.extend_from_slice(b"endstream\n");

    let limits = ResourceLimits::default().with_max_extracted_text_bytes(1);
    let scanner = StreamScanner::new(&data, limits);
    let result = scanner.scan(&mut NoopListener);
    assert!(result.is_err());
}

// --- Integration: scan with CMap in PDF ---

#[test]
fn test_scan_with_cmap_in_pdf() {
    // Build a mini-PDF with an uncompressed ToUnicode CMap stream and a
    // content stream that references the font.  Using uncompressed avoids
    // the risk of compressed data containing `\nendstream` bytes.
    let cmap_content: &[u8] = b"1 beginbfchar\n<0048> <0048>\n<0069> <0069>\nendbfchar";

    let mut data = Vec::new();

    // ToUnicode stream: object 5 0 (uncompressed).
    data.extend_from_slice(
        format!("5 0 obj\n<< /Length {} >>\nstream\n", cmap_content.len()).as_bytes(),
    );
    data.extend_from_slice(cmap_content);
    data.extend_from_slice(b"\nendstream\nendobj\n");

    // Font dictionary referencing ToUnicode.
    data.extend_from_slice(
        b"3 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /F1 /ToUnicode 5 0 R >>\nendobj\n",
    );

    // Content stream with text.
    let content: &[u8] = b"BT /F1 12 Tf (Hi) Tj ET";
    data.extend_from_slice(format!("<< /Length {} >>\nstream\n", content.len()).as_bytes());
    data.extend_from_slice(content);
    data.extend_from_slice(b"\nendstream\n");

    struct Collector {
        texts: Vec<String>,
    }
    impl PdfReadListener for Collector {
        fn on_text(&mut self, _page: usize, text: &str) -> easypdf_core::Result<()> {
            self.texts.push(text.to_string());
            Ok(())
        }
    }

    let mut collector = Collector { texts: vec![] };
    let scanner = StreamScanner::new(&data, ResourceLimits::default());
    let result = scanner.scan(&mut collector).unwrap();

    assert!(result.text_extracted);
    assert!(!collector.texts.is_empty());
    // The CMap maps 0x0048 -> H and 0x0069 -> i, so "Hi" should be
    // extracted (via CMap lookup on the font "F1").
    assert!(
        collector.texts[0].contains("Hi"),
        "expected 'Hi' in output, got: {:?}",
        collector.texts[0]
    );
}

// --- scanner: find_streams edge cases ---

#[test]
fn find_streams_no_eol_after_stream() {
    // "stream" not followed by \n or \r should be skipped.
    let data = b"streamData\nendstream";
    let streams = scanner::find_streams_for_test(data);
    assert!(streams.is_empty());
}

#[test]
fn find_streams_no_endstream() {
    let data = b"stream\ndata without endstream";
    let streams = scanner::find_streams_for_test(data);
    assert!(streams.is_empty());
}

#[test]
fn find_streams_crlf_data() {
    let data = b"stream\r\nData\r\nendstream\n";
    let streams = scanner::find_streams_for_test(data);
    assert_eq!(streams.len(), 1);
    assert_eq!(&data[streams[0].data_start..streams[0].data_end], b"Data");
}

// --- scanner: metadata edge cases ---

#[test]
fn metadata_quick_subject_keywords() {
    let data = b"<< /Subject (Test Subject) /Keywords (pdf, test) >>";
    let scanner = StreamScanner::new(data, ResourceLimits::default());
    let meta = scanner.extract_metadata_quick();
    assert_eq!(meta.subject.as_deref(), Some("Test Subject"));
    assert_eq!(meta.keywords.as_deref(), Some("pdf, test"));
}

#[test]
fn metadata_quick_creator_producer() {
    let data = b"<< /Creator (MyApp) /Producer (MyLib 1.0) >>";
    let scanner = StreamScanner::new(data, ResourceLimits::default());
    let meta = scanner.extract_metadata_quick();
    assert_eq!(meta.creator.as_deref(), Some("MyApp"));
    assert_eq!(meta.producer.as_deref(), Some("MyLib 1.0"));
}

// --- scanner: page_count edge cases ---

#[test]
fn page_count_empty() {
    let data = b"";
    let scanner = StreamScanner::new(data, ResourceLimits::default());
    assert_eq!(scanner.page_count(), 0);
}

#[test]
fn page_count_no_pages() {
    let data = b"<< /Type /Font >>";
    let scanner = StreamScanner::new(data, ResourceLimits::default());
    assert_eq!(scanner.page_count(), 0);
}

// --- scanner: scan with empty streams ---

#[test]
fn scan_no_text_streams() {
    struct Collector {
        texts: Vec<String>,
    }
    impl PdfReadListener for Collector {
        fn on_text(&mut self, _page: usize, text: &str) -> easypdf_core::Result<()> {
            self.texts.push(text.to_string());
            Ok(())
        }
    }

    // Stream with no text operators.
    let mut data = Vec::new();
    data.extend_from_slice(b"<< /Length 10 >>\nstream\n");
    data.extend_from_slice(b"q 1 0 0 1 Q\n");
    data.extend_from_slice(b"endstream\n");

    let mut collector = Collector { texts: vec![] };
    let scanner = StreamScanner::new(&data, ResourceLimits::default());
    let result = scanner.scan(&mut collector).unwrap();

    assert!(!result.text_extracted);
    assert_eq!(result.streams_processed, 1);
    assert!(collector.texts.is_empty());
}

// --- scanner: FlateDecode with array filter ---

#[test]
fn flatedecode_array_filter_detection() {
    let mut data = Vec::new();
    data.extend_from_slice(b"<< /Length 10 /Filter [/FlateDecode] >>\nstream\n");
    let stream_start = data.len();
    data.extend_from_slice(b"compressed");
    let stream_end = data.len();
    data.extend_from_slice(b"\nendstream");

    let scanner = StreamScanner::new(&data, ResourceLimits::default());
    let range = StreamRange {
        data_start: stream_start,
        data_end: stream_end,
    };
    assert!(scanner.has_flatedecode_filter(&range));
}

// --- scanner: scan with multiple streams, one empty ---

#[test]
fn scan_mixed_empty_and_text_streams() {
    struct Collector {
        texts: Vec<String>,
    }
    impl PdfReadListener for Collector {
        fn on_text(&mut self, _page: usize, text: &str) -> easypdf_core::Result<()> {
            self.texts.push(text.to_string());
            Ok(())
        }
    }

    let mut data = Vec::new();
    // Empty stream.
    data.extend_from_slice(b"<< /Length 5 >>\nstream\nq Q\n");
    data.extend_from_slice(b"endstream\n");
    // Text stream.
    data.extend_from_slice(b"<< /Length 30 >>\nstream\n");
    data.extend_from_slice(b"BT (Found) Tj ET\n");
    data.extend_from_slice(b"endstream\n");

    let mut collector = Collector { texts: vec![] };
    let scanner = StreamScanner::new(&data, ResourceLimits::default());
    let result = scanner.scan(&mut collector).unwrap();

    assert!(result.text_extracted);
    assert_eq!(result.streams_processed, 2);
    assert_eq!(result.pages_scanned, 1);
    assert!(collector.texts[0].contains("Found"));
}

// --- scanner: scan with CMap and FlateDecode (content stream only) ---

#[test]
fn scan_with_flatedecode_content_stream() {
    use flate2::write::ZlibEncoder;
    use std::io::Write;

    // Use uncompressed CMap to avoid false endstream matches.
    let cmap_content: &[u8] = b"1 beginbfchar\n<0048> <0048>\n<0069> <0069>\nendbfchar";

    let mut data = Vec::new();

    // Uncompressed ToUnicode stream.
    data.extend_from_slice(
        format!("5 0 obj\n<< /Length {} >>\nstream\n", cmap_content.len()).as_bytes(),
    );
    data.extend_from_slice(cmap_content);
    data.extend_from_slice(b"\nendstream\nendobj\n");

    // Font dictionary.
    data.extend_from_slice(
        b"3 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /F1 /ToUnicode 5 0 R >>\nendobj\n",
    );

    // Compressed content stream.
    let content: &[u8] = b"BT /F1 12 Tf (Hi) Tj ET";
    let mut encoder = ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(content).unwrap();
    let compressed = encoder.finish().unwrap();

    // Verify no false endstream in compressed data.
    assert!(
        !compressed.windows(10).any(|w| w == b"\nendstream"),
        "compressed data contains \\nendstream"
    );

    data.extend_from_slice(
        format!(
            "<< /Length {} /Filter /FlateDecode >>\nstream\n",
            compressed.len()
        )
        .as_bytes(),
    );
    data.extend_from_slice(&compressed);
    data.extend_from_slice(b"\nendstream\n");

    struct Collector {
        texts: Vec<String>,
    }
    impl PdfReadListener for Collector {
        fn on_text(&mut self, _page: usize, text: &str) -> easypdf_core::Result<()> {
            self.texts.push(text.to_string());
            Ok(())
        }
    }

    let mut collector = Collector { texts: vec![] };
    let scanner = StreamScanner::new(&data, ResourceLimits::default());
    let result = scanner.scan(&mut collector).unwrap();

    assert!(result.text_extracted);
    assert!(collector.texts[0].contains("Hi"));
}
