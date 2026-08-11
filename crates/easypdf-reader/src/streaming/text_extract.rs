//! CMap-aware text extraction from decompressed PDF content streams.
//!
//! Provides [`extract_text_with_cmap`] and supporting parse functions for
//! PDF text operators (`Tj`, `'`, `"`, `TJ`) with proper `CMap` decoding.

use std::collections::HashMap;

use super::cmap::CMap;
use super::byte_finder::{skip_whitespace, decode_octal_digit, hex_digit_value};

// ---------------------------------------------------------------------------
// CMap-aware text extraction
// ---------------------------------------------------------------------------

/// Extract text from a decompressed content stream using `CMap` mappings.
///
/// When `cmaps` is non-empty the extractor tracks the current font via `Tf`
/// operators and translates character codes through the font's `CMap`.  When
/// no `CMap` is available for the current font (or `cmaps` is empty) it
/// falls back to the legacy Latin-1 / UTF-8 byte interpretation.
///
/// Hex strings `<...>` are also decoded for CJK fonts that use hex-encoded
/// CID strings.
pub(super) fn extract_text_with_cmap(content: &[u8], cmaps: &HashMap<String, CMap>) -> String {
    let mut text = String::new();
    let len = content.len();
    let mut i = 0;
    let mut current_font: Option<String> = None;

    while i < len {
        // Track font changes: "/F1 12 Tf" or just "12 Tf" style.
        if content[i] == b'T' && i + 1 < len && content[i + 1] == b'f' {
            // Scan backwards to find the font name before "Tf".
            if let Some(font_name) = extract_current_font_name(content, i) {
                current_font = Some(font_name);
            }
            i += 2;
            continue;
        }

        // Hex string: <XXXX>
        if content[i] == b'<' && cmaps.values().any(|c| !c.is_empty())
            && let Some((hex_bytes, end)) = parse_hex_string(content, i)
        {
            let after = skip_whitespace(content, end);
            if after + 1 < len && content[after] == b'T' && content[after + 1] == b'j' {
                let decoded = decode_bytes_with_cmap(
                    &hex_bytes,
                    current_font.as_deref(),
                    cmaps,
                );
                text.push_str(&decoded);
                text.push('\n');
                i = after + 2;
                continue;
            }
            i = end;
            continue;
        }

        // Parenthesized string followed by text operator.
        if content[i] == b'('
            && let Some((string_bytes, end)) = parse_pdf_string_raw(content, i)
        {
            let after = skip_whitespace(content, end);
            if after + 1 < len {
                let decoded = decode_bytes_with_cmap(
                    &string_bytes,
                    current_font.as_deref(),
                    cmaps,
                );
                // Tj
                if content[after] == b'T' && content[after + 1] == b'j' {
                    text.push_str(&decoded);
                    text.push('\n');
                    i = after + 2;
                    continue;
                }
                // '
                if content[after] == b'\'' {
                    text.push_str(&decoded);
                    text.push('\n');
                    i = after + 1;
                    continue;
                }
                // "
                if content[after] == b'"' {
                    text.push_str(&decoded);
                    text.push('\n');
                    i = after + 1;
                    continue;
                }
            }
            i = end;
            continue;
        }

        // TJ: show string array
        if content[i] == b'['
            && let Some((strings, end)) = parse_tj_array_with_cmap(content, i, current_font.as_deref(), cmaps)
        {
            let after = skip_whitespace(content, end);
            if after + 1 < len
                && content[after] == b'T'
                && content[after + 1] == b'J'
            {
                for s in strings {
                    text.push_str(&s);
                }
                text.push('\n');
                i = after + 2;
                continue;
            }
            i = end;
            continue;
        }

        i += 1;
    }

    text
}

/// Scan backwards from a `Tf` operator to find the font name set by a prior
/// `/name size Tf` instruction.
///
/// Works correctly even when the font name ends with digits (e.g. `/F1`).
fn extract_current_font_name(content: &[u8], tf_pos: usize) -> Option<String> {
    // Pattern: ... /FontName 12 Tf
    // Strategy: scan backward to find '/', then scan forward to extract the
    // name and verify the structure.

    let mut pos = tf_pos;
    // Skip whitespace before "Tf".
    while pos > 0 && content[pos - 1].is_ascii_whitespace() {
        pos -= 1;
    }
    // Skip the font size number (digits + optional decimal).
    while pos > 0 && (content[pos - 1].is_ascii_digit() || content[pos - 1] == b'.') {
        pos -= 1;
    }
    // Skip whitespace between name and size.
    while pos > 0 && content[pos - 1].is_ascii_whitespace() {
        pos -= 1;
    }
    // `pos` now points just past the end of the font name.  Scan backward
    // to find the leading '/'.
    let name_end = pos;
    while pos > 0 && content[pos - 1] != b'/' {
        pos -= 1;
        // Stop at line boundaries -- a `/name ... Tf` must be on the same line.
        if content[pos] == b'\n' || content[pos] == b'\r' {
            return None;
        }
    }
    if pos == 0 || content[pos - 1] != b'/' {
        return None;
    }
    // `pos` points to the first char after '/'.  The name extends to `name_end`.
    if pos < name_end {
        return Some(
            String::from_utf8_lossy(&content[pos..name_end]).into_owned(),
        );
    }
    None
}

/// Decode raw bytes using the `CMap` for `font_name`, if available.
///
/// Multi-byte character codes (common in CJK fonts) are tried as 2-byte
/// big-endian codes first, then single-byte.  When no `CMap` is found the
/// bytes are decoded as lossy UTF-8 (the legacy behaviour).
fn decode_bytes_with_cmap(
    bytes: &[u8],
    font_name: Option<&str>,
    cmaps: &HashMap<String, CMap>,
) -> String {
    let cmap = font_name.and_then(|name| cmaps.get(name));

    let Some(cmap) = cmap else {
        return String::from_utf8_lossy(bytes).into_owned();
    };

    if cmap.is_empty() {
        return String::from_utf8_lossy(bytes).into_owned();
    }

    let mut result = String::new();
    let mut i = 0;

    while i < bytes.len() {
        // Try 2-byte CID first (big-endian) -- this is the common case for
        // CJK fonts whose codespace is <0000> - <FFFF>.
        if i + 1 < bytes.len() {
            let code_2 = u32::from(u16::from_be_bytes([bytes[i], bytes[i + 1]]));
            if let Some(unicode) = cmap.lookup(code_2) {
                if let Some(ch) = char::from_u32(unicode) {
                    result.push(ch);
                }
                i += 2;
                continue;
            }
        }
        // Fallback: single-byte code.
        let code_1 = u32::from(bytes[i]);
        if let Some(unicode) = cmap.lookup(code_1) {
            if let Some(ch) = char::from_u32(unicode) {
                result.push(ch);
            }
        } else {
            // No mapping -- emit as Latin-1 literal.
            if let Some(ch) = char::from_u32(u32::from(bytes[i])) {
                result.push(ch);
            }
        }
        i += 1;
    }

    result
}

/// Parse a PDF string literal and return raw bytes (no lossy UTF-8 conversion).
pub(super) fn parse_pdf_string_raw(data: &[u8], pos: usize) -> Option<(Vec<u8>, usize)> {
    if data[pos] != b'(' {
        return None;
    }

    let mut depth = 1i32;
    let mut i = pos + 1;
    let mut bytes = Vec::new();

    while i < data.len() && depth > 0 {
        match data[i] {
            b'(' => {
                depth += 1;
                bytes.push(b'(');
            }
            b')' => {
                depth -= 1;
                if depth > 0 {
                    bytes.push(b')');
                }
            }
            b'\\' => {
                i += 1;
                if i >= data.len() {
                    break;
                }
                match data[i] {
                    b'n' => bytes.push(b'\n'),
                    b'r' => bytes.push(b'\r'),
                    b't' => bytes.push(b'\t'),
                    b'\\' => bytes.push(b'\\'),
                    b'(' => bytes.push(b'('),
                    b')' => bytes.push(b')'),
                    octal @ b'0'..=b'7' => {
                        let mut val = u32::from(octal - b'0');
                        for _ in 0..2 {
                            if i + 1 < data.len() {
                                if let Some(d) = decode_octal_digit(data[i + 1]) {
                                    val = val * 8 + u32::from(d);
                                    i += 1;
                                } else {
                                    break;
                                }
                            }
                        }
                        if let Ok(byte) = u8::try_from(val & 0xFF) {
                            bytes.push(byte);
                        }
                    }
                    other => bytes.push(other),
                }
            }
            other => bytes.push(other),
        }
        i += 1;
    }

    Some((bytes, i))
}

/// Parse a hex string `<XXXX>` at position `pos`.
///
/// Returns the decoded bytes and the position after the closing `>`.
pub(super) fn parse_hex_string(data: &[u8], pos: usize) -> Option<(Vec<u8>, usize)> {
    if data[pos] != b'<' {
        return None;
    }
    let mut i = pos + 1;
    let mut hex_chars = Vec::new();

    while i < data.len() {
        match data[i] {
            b'>' => {
                // Pad odd-length hex strings with a trailing 0.
                if hex_chars.len() % 2 != 0 {
                    hex_chars.push(b'0');
                }
                let bytes = hex_chars
                    .chunks(2)
                    .filter_map(|pair| {
                        let hi = hex_digit_value(pair[0])?;
                        let lo = hex_digit_value(pair[1])?;
                        Some(hi * 16 + lo)
                    })
                    .collect();
                return Some((bytes, i + 1));
            }
            b if b.is_ascii_hexdigit() => hex_chars.push(b),
            b' ' | b'\t' | b'\n' | b'\r' => {}
            _ => return None, // invalid hex char
        }
        i += 1;
    }
    None
}

/// Parse a TJ array, applying `CMap` decoding to each string element.
fn parse_tj_array_with_cmap(
    data: &[u8],
    pos: usize,
    font_name: Option<&str>,
    cmaps: &HashMap<String, CMap>,
) -> Option<(Vec<String>, usize)> {
    if data[pos] != b'[' {
        return None;
    }

    let mut strings = Vec::new();
    let mut i = pos + 1;

    while i < data.len() {
        match data[i] {
            b']' => return Some((strings, i + 1)),
            b'(' => {
                if let Some((raw, end)) = parse_pdf_string_raw(data, i) {
                    let decoded = decode_bytes_with_cmap(&raw, font_name, cmaps);
                    strings.push(decoded);
                    i = end;
                } else {
                    i += 1;
                }
            }
            b'<' => {
                if let Some((hex_bytes, end)) = parse_hex_string(data, i) {
                    let decoded = decode_bytes_with_cmap(&hex_bytes, font_name, cmaps);
                    strings.push(decoded);
                    i = end;
                } else {
                    i += 1;
                }
            }
            _ => {
                i += 1;
            }
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Legacy text extraction (test-only)
// ---------------------------------------------------------------------------

/// Extract text from a decompressed PDF content stream (legacy, no `CMap`).
///
/// This is the backward-compatible entry point that delegates to
/// [`extract_text_with_cmap`] with an empty `CMap` table.  New code should
/// call [`extract_text_with_cmap`] directly.
#[cfg(test)]
pub(super) fn extract_text_from_content_stream(content: &[u8]) -> String {
    extract_text_with_cmap(content, &HashMap::new())
}

/// Parse a PDF string literal starting at `pos` (the opening `(`).
///
/// Returns the unescaped string content and the position after the closing `)`.
pub(super) fn parse_pdf_string(data: &[u8], pos: usize) -> Option<(String, usize)> {
    if data[pos] != b'(' {
        return None;
    }

    let mut depth = 1i32;
    let mut i = pos + 1;
    let mut bytes = Vec::new();

    while i < data.len() && depth > 0 {
        match data[i] {
            b'(' => {
                depth += 1;
                bytes.push(b'(');
            }
            b')' => {
                depth -= 1;
                if depth > 0 {
                    bytes.push(b')');
                }
            }
            b'\\' => {
                // Escape sequence.
                i += 1;
                if i >= data.len() {
                    break;
                }
                match data[i] {
                    b'n' => bytes.push(b'\n'),
                    b'r' => bytes.push(b'\r'),
                    b't' => bytes.push(b'\t'),
                    b'\\' => bytes.push(b'\\'),
                    b'(' => bytes.push(b'('),
                    b')' => bytes.push(b')'),
                    octal @ b'0'..=b'7' => {
                        // Up to 3 octal digits.
                        let mut val = u32::from(octal - b'0');
                        for _ in 0..2 {
                            if i + 1 < data.len() {
                                if let Some(d) = decode_octal_digit(data[i + 1]) {
                                    val = val * 8 + u32::from(d);
                                    i += 1;
                                } else {
                                    break;
                                }
                            }
                        }
                        if let Ok(byte) = u8::try_from(val & 0xFF) {
                            bytes.push(byte);
                        }
                    }
                    other => bytes.push(other),
                }
            }
            other => bytes.push(other),
        }
        i += 1;
    }

    let s = String::from_utf8_lossy(&bytes).into_owned();
    Some((s, i))
}

/// Parse a TJ array starting at `pos` (the opening `[`).
///
/// Returns a list of string contents extracted from the array.
#[cfg(test)]
pub(super) fn parse_tj_array(data: &[u8], pos: usize) -> Option<(Vec<String>, usize)> {
    if data[pos] != b'[' {
        return None;
    }

    let mut strings = Vec::new();
    let mut i = pos + 1;

    while i < data.len() {
        match data[i] {
            b']' => {
                return Some((strings, i + 1));
            }
            b'(' => {
                if let Some((s, end)) = parse_pdf_string(data, i) {
                    strings.push(s);
                    i = end;
                } else {
                    i += 1;
                }
            }
            _ => {
                // Skip numbers, names, whitespace, etc.
                i += 1;
            }
        }
    }

    None
}
