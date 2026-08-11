//! Tests for the streaming PDF scanner.
//!
//! All 33 tests from the original `streaming.rs` are consolidated here.

#![allow(clippy::items_after_statements)]

use std::collections::HashMap;

use easypdf_core::{PdfReadListener, ResourceLimits};

use super::cmap::CMap;
use super::scanner::{StreamRange, StreamScanner};
use super::text_extract::{
    extract_text_with_cmap, parse_hex_string, parse_pdf_string,
};
#[cfg(test)]
use super::text_extract::{extract_text_from_content_stream, parse_tj_array};
use super::scanner;

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
    assert_eq!(
        &data[streams[0].data_start..streams[0].data_end],
        b"AAA"
    );
    assert_eq!(
        &data[streams[1].data_start..streams[1].data_end],
        b"BBB"
    );
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
    assert_eq!(
        &data[streams[0].data_start..streams[0].data_end],
        b"Hello"
    );
}

// --- Text extraction from content streams ---

#[test]
fn extract_text_tj() {
    let content = b"BT /F1 12 Tf 72 700 Td (Hello World) Tj ET";
    let text = extract_text_from_content_stream(content);
    assert_eq!(text, "Hello World\n");
}

#[test]
fn extract_text_tj_multiple() {
    let content = b"BT (Hello) Tj (World) Tj ET";
    let text = extract_text_from_content_stream(content);
    assert_eq!(text, "Hello\nWorld\n");
}

#[test]
fn extract_text_tj_with_escapes() {
    let content = b"BT (Line1\\nLine2) Tj ET";
    let text = extract_text_from_content_stream(content);
    assert_eq!(text, "Line1\nLine2\n");
}

#[test]
fn extract_text_tj_apostrophe() {
    let content = b"BT (First) Tj (Second)' ET";
    let text = extract_text_from_content_stream(content);
    assert_eq!(text, "First\nSecond\n");
}

#[test]
fn extract_text_tj_double_quote() {
    let content = b"BT (word) \" (text) Tj ET";
    let text = extract_text_from_content_stream(content);
    // The " operator expects two operands before it, but our simplified
    // parser just extracts the string.
    assert!(text.contains("word"));
}

#[test]
fn extract_text_tj_array() {
    let content = b"BT [(Hello) 12 (World)] TJ ET";
    let text = extract_text_from_content_stream(content);
    assert_eq!(text, "HelloWorld\n");
}

#[test]
fn extract_text_empty_stream() {
    let content = b"BT ET";
    let text = extract_text_from_content_stream(content);
    assert!(text.is_empty());
}

#[test]
fn extract_text_nested_parens() {
    let content = b"BT (Hello (nested)) Tj ET";
    let text = extract_text_from_content_stream(content);
    assert_eq!(text, "Hello (nested)\n");
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

// --- PDF string parsing ---

#[test]
fn parse_pdf_string_simple() {
    let data = b"(Hello)";
    let (s, end) = parse_pdf_string(data, 0).unwrap();
    assert_eq!(s, "Hello");
    assert_eq!(end, 7);
}

#[test]
fn parse_pdf_string_nested() {
    let data = b"(Hello (World))";
    let (s, end) = parse_pdf_string(data, 0).unwrap();
    assert_eq!(s, "Hello (World)");
    assert_eq!(end, 15);
}

#[test]
fn parse_pdf_string_escaped() {
    let data = b"(Line1\\nLine2)";
    let (s, _) = parse_pdf_string(data, 0).unwrap();
    assert_eq!(s, "Line1\nLine2");
}

#[test]
fn parse_pdf_string_octal() {
    let data = b"(\\110\\151)"; // "Hi" in octal
    let (s, _) = parse_pdf_string(data, 0).unwrap();
    assert_eq!(s, "Hi");
}

// --- TJ array parsing ---

#[test]
fn parse_tj_array_simple() {
    let data = b"[(Hello) 10 (World)]";
    let (strings, end) = parse_tj_array(data, 0).unwrap();
    assert_eq!(strings, vec!["Hello", "World"]);
    assert_eq!(end, 20);
}

#[test]
fn parse_tj_array_empty() {
    let data = b"[]";
    let (strings, end) = parse_tj_array(data, 0).unwrap();
    assert!(strings.is_empty());
    assert_eq!(end, 2);
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

// --- CMap parsing ---

#[test]
fn test_cmap_parse_bfchar() {
    let cmap_data = b"1 beginbfchar\n<0041> <00E9>\nendbfchar";
    let cmap = CMap::parse(cmap_data);
    assert_eq!(cmap.lookup(0x0041), Some(0x00E9)); // e-acute
    assert!(cmap.lookup(0x0000).is_none());
}

#[test]
fn test_cmap_parse_bfchar_multiple_entries() {
    let cmap_data = b"3 beginbfchar\n<0041> <0041>\n<0042> <0042>\n<0043> <0043>\nendbfchar";
    let cmap = CMap::parse(cmap_data);
    assert_eq!(cmap.lookup(0x0041), Some('A' as u32));
    assert_eq!(cmap.lookup(0x0042), Some('B' as u32));
    assert_eq!(cmap.lookup(0x0043), Some('C' as u32));
    assert!(cmap.lookup(0x0044).is_none());
}

#[test]
fn test_cmap_parse_bfrange() {
    let cmap_data = b"1 beginbfrange\n<0000> <0002> <0041>\nendbfrange";
    let cmap = CMap::parse(cmap_data);
    assert_eq!(cmap.lookup(0x0000), Some(0x0041)); // A
    assert_eq!(cmap.lookup(0x0001), Some(0x0042)); // B
    assert_eq!(cmap.lookup(0x0002), Some(0x0043)); // C
    assert!(cmap.lookup(0x0003).is_none());
}

#[test]
fn test_cmap_parse_combined() {
    let cmap_data = b"1 beginbfchar\n<0041> <00E9>\nendbfchar\n1 beginbfrange\n<0000> <0002> <0041>\nendbfrange";
    let cmap = CMap::parse(cmap_data);
    // bfchar takes priority over bfrange for the same code.
    assert_eq!(cmap.lookup(0x0041), Some(0x00E9));
    // bfrange entries are still accessible.
    assert_eq!(cmap.lookup(0x0000), Some(0x0041));
}

#[test]
fn test_cmap_lookup_empty() {
    let cmap = CMap::parse(b"");
    assert!(cmap.is_empty());
    assert!(cmap.lookup(0x0041).is_none());
}

#[test]
fn test_cmap_bfchar_with_spaces() {
    // Some CMap producers use multiple spaces between hex tokens.
    let cmap_data = b"1 beginbfchar\n<3000> <3000>   <3001> <3001>\nendbfchar";
    let cmap = CMap::parse(cmap_data);
    assert_eq!(cmap.lookup(0x3000), Some(0x3000));
    assert_eq!(cmap.lookup(0x3001), Some(0x3001));
}

// --- CMap-aware text extraction ---

#[test]
fn test_extract_text_with_cmap_bfchar() {
    let mut cmaps = HashMap::new();
    let cmap = CMap::parse(b"1 beginbfchar\n<0041> <00E9>\nendbfchar");
    cmaps.insert("F1".to_owned(), cmap);

    // Content: /F1 12 Tf (A) Tj   -- byte 0x41 should map to U+00E9
    let content = b"/F1 12 Tf (A) Tj";
    let text = extract_text_with_cmap(content, &cmaps);
    assert!(text.contains('\u{00E9}'));
}

#[test]
fn test_extract_text_with_cmap_two_byte_cid() {
    let mut cmaps = HashMap::new();
    // CJK-style: 2-byte CID <4E2D> maps to U+4E2D (中)
    let cmap = CMap::parse(b"1 beginbfchar\n<4E2D> <4E2D>\nendbfchar");
    cmaps.insert("F1".to_owned(), cmap);

    // Content: /F1 12 Tf (<4E2D>) Tj  -- encoded as 2 raw bytes 0x4E 0x2D
    let content = b"/F1 12 Tf (\x4E\x2D) Tj";
    let text = extract_text_with_cmap(content, &cmaps);
    assert!(text.contains('\u{4E2D}'), "expected 中, got: {text:?}");
}

#[test]
fn test_extract_text_with_cmap_hex_string() {
    let mut cmaps = HashMap::new();
    let cmap = CMap::parse(b"1 beginbfchar\n<4E2D> <4E2D>\nendbfchar");
    cmaps.insert("F1".to_owned(), cmap);

    // Hex string: <4E2D> Tj
    let content = b"/F1 12 Tf <4E2D> Tj";
    let text = extract_text_with_cmap(content, &cmaps);
    assert!(text.contains('\u{4E2D}'), "expected 中 from hex, got: {text:?}");
}

#[test]
fn test_extract_text_no_cmap_fallback() {
    let cmaps: HashMap<String, CMap> = HashMap::new();
    let content = b"BT /F1 12 Tf (Hello) Tj ET";
    let text = extract_text_with_cmap(content, &cmaps);
    assert_eq!(text, "Hello\n");
}

#[test]
fn test_extract_text_cmap_bfrange_cjk() {
    let mut cmaps = HashMap::new();
    // Map range 0x4E00-0x4E02 to Unicode U+4E00-U+4E02 (一 丁 七)
    let cmap = CMap::parse(b"1 beginbfrange\n<4E00> <4E02> <4E00>\nendbfrange");
    cmaps.insert("F1".to_owned(), cmap);

    // Three 2-byte CIDs: 0x4E00, 0x4E01, 0x4E02
    let content = b"/F1 12 Tf (\x4E\x00\x4E\x01\x4E\x02) Tj";
    let text = extract_text_with_cmap(content, &cmaps);
    assert!(text.contains('\u{4E00}'), "expected 一, got: {text:?}");
    assert!(text.contains('\u{4E01}'), "expected 丁, got: {text:?}");
    assert!(text.contains('\u{4E02}'), "expected 七, got: {text:?}");
}

// --- Hex string parsing ---

#[test]
fn test_parse_hex_string_simple() {
    let data = b"<4E2D>";
    let (bytes, end) = parse_hex_string(data, 0).unwrap();
    assert_eq!(bytes, vec![0x4E, 0x2D]);
    assert_eq!(end, 6);
}

#[test]
fn test_parse_hex_string_odd_length() {
    let data = b"<ABC>";
    let (bytes, end) = parse_hex_string(data, 0).unwrap();
    // Odd-length hex gets padded: ABC0 -> AB C0
    assert_eq!(bytes, vec![0xAB, 0xC0]);
    assert_eq!(end, 5);
}

#[test]
fn test_parse_hex_string_with_spaces() {
    let data = b"< 4E 2D >";
    let (bytes, end) = parse_hex_string(data, 0).unwrap();
    assert_eq!(bytes, vec![0x4E, 0x2D]);
    assert_eq!(end, 9);
}

// --- Font name extraction ---

#[test]
fn test_extract_font_name_from_basefont() {
    let region = "/Type /Font /Subtype /Type0 /BaseFont /ABCDEE+SimSun /Encoding /Identity-H";
    let name = scanner::extract_font_name_for_test(region, 0);
    assert_eq!(name, "ABCDEE+SimSun");
}

#[test]
fn test_extract_font_name_fallback() {
    let region = "/Type /Font /Subtype /Type1";
    let name = scanner::extract_font_name_for_test(region, 42);
    assert_eq!(name, "font_42");
}

// --- ToUnicode reference extraction ---

#[test]
fn test_extract_to_unicode_ref() {
    let region = "/Type /Font /ToUnicode 12 0 R /BaseFont /F1";
    let result = scanner::extract_to_unicode_ref_for_test(region);
    assert_eq!(result, Some((12, 0)));
}

#[test]
fn test_extract_to_unicode_ref_missing() {
    let region = "/Type /Font /BaseFont /F1";
    let result = scanner::extract_to_unicode_ref_for_test(region);
    assert_eq!(result, None);
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
        format!(
            "5 0 obj\n<< /Length {} >>\nstream\n",
            cmap_content.len()
        )
        .as_bytes(),
    );
    data.extend_from_slice(cmap_content);
    data.extend_from_slice(b"\nendstream\nendobj\n");

    // Font dictionary referencing ToUnicode.
    data.extend_from_slice(
        b"3 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /F1 /ToUnicode 5 0 R >>\nendobj\n",
    );

    // Content stream with text.
    let content: &[u8] = b"BT /F1 12 Tf (Hi) Tj ET";
    data.extend_from_slice(
        format!("<< /Length {} >>\nstream\n", content.len()).as_bytes(),
    );
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

// ===========================================================================
// Additional coverage tests for byte_finder, text_extract, and scanner
// ===========================================================================

// --- byte_finder: find_keyword edge cases ---

#[test]
fn find_keyword_empty_keyword_returns_none() {
    let data = b"hello world";
    assert!(super::byte_finder::find_keyword(data, 0, b"").is_none());
}

#[test]
fn find_keyword_from_beyond_data_len() {
    let data = b"hello";
    assert!(super::byte_finder::find_keyword(data, 100, b"hello").is_none());
}

#[test]
fn find_keyword_at_end() {
    let data = b"abcendstream";
    let pos = super::byte_finder::find_keyword(data, 0, b"endstream");
    assert_eq!(pos, Some(3));
}

#[test]
fn find_keyword_not_found() {
    let data = b"hello world";
    assert!(super::byte_finder::find_keyword(data, 0, b"xyz").is_none());
}

#[test]
fn find_keyword_from_offset() {
    let data = b"abcabc";
    let pos = super::byte_finder::find_keyword(data, 1, b"abc");
    assert_eq!(pos, Some(3));
}

// --- byte_finder: find_endstream edge cases ---

#[test]
fn find_endstream_from_beyond_data() {
    let data = b"\nendstream";
    assert!(super::byte_finder::find_endstream(data, 100).is_none());
}

#[test]
fn find_endstream_with_cr() {
    let data = b"data\rendstream\n";
    let pos = super::byte_finder::find_endstream(data, 0);
    assert_eq!(pos, Some(5));
}

#[test]
fn find_endstream_no_match() {
    let data = b"data without the keyword";
    assert!(super::byte_finder::find_endstream(data, 0).is_none());
}

#[test]
fn find_endstream_followed_by_eof() {
    let data = b"data\nendstream";
    let pos = super::byte_finder::find_endstream(data, 0);
    assert_eq!(pos, Some(5));
}

#[test]
fn find_endstream_false_positive_in_data() {
    // "endstream" appears in the data but not preceded by \n.
    let data = b"dataendstream\n";
    assert!(super::byte_finder::find_endstream(data, 0).is_none());
}

// --- byte_finder: skip_whitespace ---

#[test]
fn skip_whitespace_all_types() {
    let data = b" \t\n\r\x00hello";
    let pos = super::byte_finder::skip_whitespace(data, 0);
    assert_eq!(pos, 5);
}

#[test]
fn skip_whitespace_at_end() {
    let data = b"hello   ";
    let pos = super::byte_finder::skip_whitespace(data, 5);
    assert_eq!(pos, 8);
}

#[test]
fn skip_whitespace_no_whitespace() {
    let data = b"hello";
    let pos = super::byte_finder::skip_whitespace(data, 0);
    assert_eq!(pos, 0);
}

// --- byte_finder: decode_octal_digit ---

#[test]
fn decode_octal_digit_valid() {
    assert_eq!(super::byte_finder::decode_octal_digit(b'0'), Some(0));
    assert_eq!(super::byte_finder::decode_octal_digit(b'7'), Some(7));
    assert_eq!(super::byte_finder::decode_octal_digit(b'3'), Some(3));
}

#[test]
fn decode_octal_digit_invalid() {
    assert!(super::byte_finder::decode_octal_digit(b'8').is_none());
    assert!(super::byte_finder::decode_octal_digit(b'a').is_none());
}

// --- byte_finder: hex_digit_value ---

#[test]
fn hex_digit_value_digits() {
    assert_eq!(super::byte_finder::hex_digit_value(b'0'), Some(0));
    assert_eq!(super::byte_finder::hex_digit_value(b'9'), Some(9));
}

#[test]
fn hex_digit_value_lower_hex() {
    assert_eq!(super::byte_finder::hex_digit_value(b'a'), Some(10));
    assert_eq!(super::byte_finder::hex_digit_value(b'f'), Some(15));
}

#[test]
fn hex_digit_value_upper_hex() {
    assert_eq!(super::byte_finder::hex_digit_value(b'A'), Some(10));
    assert_eq!(super::byte_finder::hex_digit_value(b'F'), Some(15));
}

#[test]
fn hex_digit_value_invalid() {
    assert!(super::byte_finder::hex_digit_value(b'g').is_none());
    assert!(super::byte_finder::hex_digit_value(b'z').is_none());
    assert!(super::byte_finder::hex_digit_value(b' ').is_none());
}

// --- byte_finder: usize_to_u64 ---

#[test]
fn usize_to_u64_normal() {
    assert_eq!(super::byte_finder::usize_to_u64(42), 42);
    assert_eq!(super::byte_finder::usize_to_u64(0), 0);
}

// --- text_extract: additional text extraction tests ---

#[test]
fn extract_text_with_cmap_empty_content() {
    let cmaps: HashMap<String, CMap> = HashMap::new();
    let text = extract_text_with_cmap(b"", &cmaps);
    assert!(text.is_empty());
}

#[test]
fn extract_text_with_cmap_only_operators() {
    let cmaps: HashMap<String, CMap> = HashMap::new();
    let content = b"BT ET";
    let text = extract_text_with_cmap(content, &cmaps);
    assert!(text.is_empty());
}

#[test]
fn extract_text_with_cmap_font_switch() {
    let mut cmaps = HashMap::new();
    let cmap1 = CMap::parse(b"1 beginbfchar\n<0041> <0041>\nendbfchar");
    let cmap2 = CMap::parse(b"1 beginbfchar\n<0042> <00E9>\nendbfchar");
    cmaps.insert("F1".to_owned(), cmap1);
    cmaps.insert("F2".to_owned(), cmap2);

    // Switch font from F1 to F2 mid-stream.
    let content = b"/F1 12 Tf (A) Tj /F2 12 Tf (B) Tj";
    let text = extract_text_with_cmap(content, &cmaps);
    assert!(text.contains('A'), "expected A from F1, got: {text:?}");
    assert!(text.contains('\u{00E9}'), "expected e-acute from F2, got: {text:?}");
}

#[test]
fn extract_text_with_cmap_tj_array_with_hex() {
    let mut cmaps = HashMap::new();
    let cmap = CMap::parse(b"1 beginbfchar\n<4E2D> <4E2D>\nendbfchar");
    cmaps.insert("F1".to_owned(), cmap);

    // TJ array with hex strings.
    let content = b"/F1 12 Tf [<4E2D> 0 <4E2D>] TJ";
    let text = extract_text_with_cmap(content, &cmaps);
    assert!(text.contains('\u{4E2D}'), "expected 中 in TJ array, got: {text:?}");
}

#[test]
fn extract_text_with_cmap_unknown_font_fallback() {
    let mut cmaps = HashMap::new();
    let cmap = CMap::parse(b"1 beginbfchar\n<0041> <00E9>\nendbfchar");
    cmaps.insert("F1".to_owned(), cmap);

    // Use font F2 which has no CMap -- should fall back to UTF-8.
    let content = b"/F2 12 Tf (Hello) Tj";
    let text = extract_text_with_cmap(content, &cmaps);
    assert_eq!(text, "Hello\n");
}

#[test]
fn extract_text_with_cmap_cmap_empty_fallback() {
    let mut cmaps = HashMap::new();
    cmaps.insert("F1".to_owned(), CMap::parse(b""));

    let content = b"/F1 12 Tf (Hello) Tj";
    let text = extract_text_with_cmap(content, &cmaps);
    assert_eq!(text, "Hello\n");
}

#[test]
fn extract_text_with_cmap_single_byte_in_cmap() {
    let mut cmaps = HashMap::new();
    // Map single byte 0x41 -> U+00E9
    let cmap = CMap::parse(b"1 beginbfchar\n<0041> <00E9>\nendbfchar");
    cmaps.insert("F1".to_owned(), cmap);

    // Single byte 0x41 (not a valid 2-byte code for this cmap).
    let content = b"/F1 12 Tf (\x41) Tj";
    let text = extract_text_with_cmap(content, &cmaps);
    assert!(text.contains('\u{00E9}'), "expected e-acute, got: {text:?}");
}

#[test]
fn extract_text_with_cmap_bytes_no_mapping_latin1() {
    let mut cmaps = HashMap::new();
    // CMap with no entries for high bytes.
    let cmap = CMap::parse(b"1 beginbfchar\n<0041> <0041>\nendbfchar");
    cmaps.insert("F1".to_owned(), cmap);

    // Byte 0xFF has no mapping -- should emit as Latin-1 literal.
    let content = b"/F1 12 Tf (\xFF) Tj";
    let text = extract_text_with_cmap(content, &cmaps);
    assert!(!text.is_empty(), "expected some output for unmapped byte");
}

// --- text_extract: parse_pdf_string_raw edge cases ---

#[test]
fn parse_pdf_string_raw_not_paren() {
    let data = b"hello";
    assert!(super::text_extract::parse_pdf_string_raw(data, 0).is_none());
}

#[test]
fn parse_pdf_string_raw_escape_at_end() {
    let data = b"(hello\\";
    let (bytes, _) = super::text_extract::parse_pdf_string_raw(data, 0).unwrap();
    assert_eq!(bytes, b"hello");
}

#[test]
fn parse_pdf_string_raw_octal_two_digits() {
    let data = b"(\\12)";
    let (bytes, _) = super::text_extract::parse_pdf_string_raw(data, 0).unwrap();
    assert_eq!(bytes[0], 10); // \12 = 10 in octal
}

#[test]
fn parse_pdf_string_raw_octal_one_digit() {
    let data = b"(\\7)";
    let (bytes, _) = super::text_extract::parse_pdf_string_raw(data, 0).unwrap();
    assert_eq!(bytes[0], 7);
}

#[test]
fn parse_pdf_string_raw_unknown_escape() {
    let data = b"(\\z)";
    let (bytes, _) = super::text_extract::parse_pdf_string_raw(data, 0).unwrap();
    assert_eq!(bytes[0], b'z'); // unknown escape passes through
}

// --- text_extract: parse_hex_string edge cases ---

#[test]
fn parse_hex_string_not_angle_bracket() {
    let data = b"hello";
    assert!(super::text_extract::parse_hex_string(data, 0).is_none());
}

#[test]
fn parse_hex_string_empty() {
    let data = b"<>";
    let (bytes, end) = super::text_extract::parse_hex_string(data, 0).unwrap();
    assert!(bytes.is_empty());
    assert_eq!(end, 2);
}

#[test]
fn parse_hex_string_invalid_char() {
    let data = b"<4G>";
    assert!(super::text_extract::parse_hex_string(data, 0).is_none());
}

#[test]
fn parse_hex_string_unclosed() {
    let data = b"<4E2D";
    assert!(super::text_extract::parse_hex_string(data, 0).is_none());
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
    assert_eq!(
        &data[streams[0].data_start..streams[0].data_end],
        b"Data"
    );
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

// --- text_extract: parse_pdf_string edge cases ---

#[test]
fn parse_pdf_string_not_paren() {
    let data = b"hello";
    assert!(parse_pdf_string(data, 0).is_none());
}

#[test]
fn parse_pdf_string_empty() {
    let data = b"()";
    let (s, end) = parse_pdf_string(data, 0).unwrap();
    assert_eq!(s, "");
    assert_eq!(end, 2);
}

#[test]
fn parse_pdf_string_backslash_at_end() {
    let data = b"(hello\\";
    let (s, _) = parse_pdf_string(data, 0).unwrap();
    assert_eq!(s, "hello");
}

// --- text_extract: TJ array with hex strings in CMap mode ---

#[test]
fn test_tj_array_with_hex_and_paren() {
    let mut cmaps = HashMap::new();
    let cmap = CMap::parse(b"1 beginbfchar\n<0041> <0041>\nendbfchar");
    cmaps.insert("F1".to_owned(), cmap);

    // TJ array mixing parenthesized and hex strings.
    let content = b"/F1 12 Tf [(Hello) <0041> (World)] TJ";
    let text = extract_text_with_cmap(content, &cmaps);
    assert!(text.contains("Hello"), "expected Hello, got: {text:?}");
    assert!(text.contains('A'), "expected A from hex, got: {text:?}");
    assert!(text.contains("World"), "expected World, got: {text:?}");
}

// --- text_extract: double-quote operator ---

#[test]
fn extract_text_double_quote_operator() {
    let cmaps: HashMap<String, CMap> = HashMap::new();
    let content = b"BT 0 0 (word) \" ET";
    let text = extract_text_with_cmap(content, &cmaps);
    assert!(text.contains("word"), "expected word from \" operator, got: {text:?}");
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

// --- text_extract: decode_bytes_with_cmap single-byte then two-byte ---

#[test]
fn decode_mixed_single_and_two_byte_cmap() {
    let mut cmaps = HashMap::new();
    // Map 0x41 -> A (single byte) and 0x4E2D -> 中 (two byte).
    let cmap = CMap::parse(b"2 beginbfchar\n<0041> <0041>\n<4E2D> <4E2D>\nendbfchar");
    cmaps.insert("F1".to_owned(), cmap);

    // Single byte 0x41 followed by two-byte 0x4E 0x2D.
    let content = b"/F1 12 Tf (\x41\x4E\x2D) Tj";
    let text = extract_text_with_cmap(content, &cmaps);
    assert!(text.contains('A'), "expected A, got: {text:?}");
    assert!(text.contains('\u{4E2D}'), "expected 中, got: {text:?}");
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
        format!(
            "5 0 obj\n<< /Length {} >>\nstream\n",
            cmap_content.len()
        )
        .as_bytes(),
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
