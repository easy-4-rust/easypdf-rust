use std::collections::HashMap;

use super::super::cmap::CMap;
use super::super::scanner;
use super::super::text_extract::{extract_text_with_cmap, parse_hex_string};

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
    assert!(
        text.contains('\u{4E2D}'),
        "expected 中 from hex, got: {text:?}"
    );
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

// --- text_extract: additional CMap text extraction tests ---

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
    assert!(
        text.contains('\u{00E9}'),
        "expected e-acute from F2, got: {text:?}"
    );
}

#[test]
fn extract_text_with_cmap_tj_array_with_hex() {
    let mut cmaps = HashMap::new();
    let cmap = CMap::parse(b"1 beginbfchar\n<4E2D> <4E2D>\nendbfchar");
    cmaps.insert("F1".to_owned(), cmap);

    // TJ array with hex strings.
    let content = b"/F1 12 Tf [<4E2D> 0 <4E2D>] TJ";
    let text = extract_text_with_cmap(content, &cmaps);
    assert!(
        text.contains('\u{4E2D}'),
        "expected 中 in TJ array, got: {text:?}"
    );
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
    assert!(
        text.contains("word"),
        "expected word from \" operator, got: {text:?}"
    );
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
