// --- byte_finder: find_keyword edge cases ---

#[test]
fn find_keyword_empty_keyword_returns_none() {
    let data = b"hello world";
    assert!(super::super::byte_finder::find_keyword(data, 0, b"").is_none());
}

#[test]
fn find_keyword_from_beyond_data_len() {
    let data = b"hello";
    assert!(super::super::byte_finder::find_keyword(data, 100, b"hello").is_none());
}

#[test]
fn find_keyword_at_end() {
    let data = b"abcendstream";
    let pos = super::super::byte_finder::find_keyword(data, 0, b"endstream");
    assert_eq!(pos, Some(3));
}

#[test]
fn find_keyword_not_found() {
    let data = b"hello world";
    assert!(super::super::byte_finder::find_keyword(data, 0, b"xyz").is_none());
}

#[test]
fn find_keyword_from_offset() {
    let data = b"abcabc";
    let pos = super::super::byte_finder::find_keyword(data, 1, b"abc");
    assert_eq!(pos, Some(3));
}

// --- byte_finder: find_endstream edge cases ---

#[test]
fn find_endstream_from_beyond_data() {
    let data = b"\nendstream";
    assert!(super::super::byte_finder::find_endstream(data, 100).is_none());
}

#[test]
fn find_endstream_with_cr() {
    let data = b"data\rendstream\n";
    let pos = super::super::byte_finder::find_endstream(data, 0);
    assert_eq!(pos, Some(5));
}

#[test]
fn find_endstream_no_match() {
    let data = b"data without the keyword";
    assert!(super::super::byte_finder::find_endstream(data, 0).is_none());
}

#[test]
fn find_endstream_followed_by_eof() {
    let data = b"data\nendstream";
    let pos = super::super::byte_finder::find_endstream(data, 0);
    assert_eq!(pos, Some(5));
}

#[test]
fn find_endstream_false_positive_in_data() {
    // "endstream" appears in the data but not preceded by \n.
    let data = b"dataendstream\n";
    assert!(super::super::byte_finder::find_endstream(data, 0).is_none());
}

// --- byte_finder: skip_whitespace ---

#[test]
fn skip_whitespace_all_types() {
    let data = b" \t\n\r\x00hello";
    let pos = super::super::byte_finder::skip_whitespace(data, 0);
    assert_eq!(pos, 5);
}

#[test]
fn skip_whitespace_at_end() {
    let data = b"hello   ";
    let pos = super::super::byte_finder::skip_whitespace(data, 5);
    assert_eq!(pos, 8);
}

#[test]
fn skip_whitespace_no_whitespace() {
    let data = b"hello";
    let pos = super::super::byte_finder::skip_whitespace(data, 0);
    assert_eq!(pos, 0);
}

// --- byte_finder: decode_octal_digit ---

#[test]
fn decode_octal_digit_valid() {
    assert_eq!(super::super::byte_finder::decode_octal_digit(b'0'), Some(0));
    assert_eq!(super::super::byte_finder::decode_octal_digit(b'7'), Some(7));
    assert_eq!(super::super::byte_finder::decode_octal_digit(b'3'), Some(3));
}

#[test]
fn decode_octal_digit_invalid() {
    assert!(super::super::byte_finder::decode_octal_digit(b'8').is_none());
    assert!(super::super::byte_finder::decode_octal_digit(b'a').is_none());
}

// --- byte_finder: hex_digit_value ---

#[test]
fn hex_digit_value_digits() {
    assert_eq!(super::super::byte_finder::hex_digit_value(b'0'), Some(0));
    assert_eq!(super::super::byte_finder::hex_digit_value(b'9'), Some(9));
}

#[test]
fn hex_digit_value_lower_hex() {
    assert_eq!(super::super::byte_finder::hex_digit_value(b'a'), Some(10));
    assert_eq!(super::super::byte_finder::hex_digit_value(b'f'), Some(15));
}

#[test]
fn hex_digit_value_upper_hex() {
    assert_eq!(super::super::byte_finder::hex_digit_value(b'A'), Some(10));
    assert_eq!(super::super::byte_finder::hex_digit_value(b'F'), Some(15));
}

#[test]
fn hex_digit_value_invalid() {
    assert!(super::super::byte_finder::hex_digit_value(b'g').is_none());
    assert!(super::super::byte_finder::hex_digit_value(b'z').is_none());
    assert!(super::super::byte_finder::hex_digit_value(b' ').is_none());
}

// --- byte_finder: usize_to_u64 ---

#[test]
fn usize_to_u64_normal() {
    assert_eq!(super::super::byte_finder::usize_to_u64(42), 42);
    assert_eq!(super::super::byte_finder::usize_to_u64(0), 0);
}
