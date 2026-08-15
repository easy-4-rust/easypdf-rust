use super::super::text_extract::{
    extract_text_from_content_stream, parse_hex_string, parse_pdf_string, parse_pdf_string_raw,
    parse_tj_array,
};

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

// --- text_extract: parse_pdf_string_raw edge cases ---

#[test]
fn parse_pdf_string_raw_not_paren() {
    let data = b"hello";
    assert!(parse_pdf_string_raw(data, 0).is_none());
}

#[test]
fn parse_pdf_string_raw_escape_at_end() {
    let data = b"(hello\\";
    let (bytes, _) = parse_pdf_string_raw(data, 0).unwrap();
    assert_eq!(bytes, b"hello");
}

#[test]
fn parse_pdf_string_raw_octal_two_digits() {
    let data = b"(\\12)";
    let (bytes, _) = parse_pdf_string_raw(data, 0).unwrap();
    assert_eq!(bytes[0], 10); // \12 = 10 in octal
}

#[test]
fn parse_pdf_string_raw_octal_one_digit() {
    let data = b"(\\7)";
    let (bytes, _) = parse_pdf_string_raw(data, 0).unwrap();
    assert_eq!(bytes[0], 7);
}

#[test]
fn parse_pdf_string_raw_unknown_escape() {
    let data = b"(\\z)";
    let (bytes, _) = parse_pdf_string_raw(data, 0).unwrap();
    assert_eq!(bytes[0], b'z'); // unknown escape passes through
}

// --- text_extract: parse_hex_string edge cases ---

#[test]
fn parse_hex_string_not_angle_bracket() {
    let data = b"hello";
    assert!(parse_hex_string(data, 0).is_none());
}

#[test]
fn parse_hex_string_empty() {
    let data = b"<>";
    let (bytes, end) = parse_hex_string(data, 0).unwrap();
    assert!(bytes.is_empty());
    assert_eq!(end, 2);
}

#[test]
fn parse_hex_string_invalid_char() {
    let data = b"<4G>";
    assert!(parse_hex_string(data, 0).is_none());
}

#[test]
fn parse_hex_string_unclosed() {
    let data = b"<4E2D";
    assert!(parse_hex_string(data, 0).is_none());
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
