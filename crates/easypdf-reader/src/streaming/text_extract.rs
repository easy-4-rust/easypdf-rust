//! 从解压后的 PDF 内容流中进行 `CMap` 感知的文本提取。
//!
//! 提供 [`extract_text_with_cmap`] 及其辅助解析函数，用于
//! PDF 文本算子（`Tj`、`'`、`"`、`TJ`）的正确 `CMap` 解码。

use std::collections::HashMap;

use super::byte_finder::{decode_octal_digit, hex_digit_value, skip_whitespace};
use super::cmap::CMap;

// ---------------------------------------------------------------------------
// CMap-aware text extraction
// ---------------------------------------------------------------------------

/// 使用 `CMap` 映射从解压后的内容流中提取文本。
///
/// 当 `cmaps` 非空时，提取器通过 `Tf` 算子跟踪当前字体，
/// 并通过字体的 `CMap` 转换字符码。当当前字体没有可用的 `CMap`
/// （或 `cmaps` 为空）时，回退到传统的 Latin-1 / UTF-8 字节解释。
///
/// 十六进制字符串 `<...>` 也会被解码，用于使用十六进制编码
/// CID 字符串的 CJK 字体。
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
        if content[i] == b'<'
            && cmaps.values().any(|c| !c.is_empty())
            && let Some((hex_bytes, end)) = parse_hex_string(content, i)
        {
            let after = skip_whitespace(content, end);
            if after + 1 < len && content[after] == b'T' && content[after + 1] == b'j' {
                let decoded = decode_bytes_with_cmap(&hex_bytes, current_font.as_deref(), cmaps);
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
                let decoded = decode_bytes_with_cmap(&string_bytes, current_font.as_deref(), cmaps);
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
            && let Some((strings, end)) =
                parse_tj_array_with_cmap(content, i, current_font.as_deref(), cmaps)
        {
            let after = skip_whitespace(content, end);
            if after + 1 < len && content[after] == b'T' && content[after + 1] == b'J' {
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

/// 从 `Tf` 算子向后扫描以查找先前 `/name size Tf` 指令设置的字体名。
///
/// 即使字体名以数字结尾（如 `/F1`）也能正确工作。
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
        return Some(String::from_utf8_lossy(&content[pos..name_end]).into_owned());
    }
    None
}

/// 使用 `font_name` 对应的 `CMap` 解码原始字节（如可用）。
///
/// 多字节字符码（CJK 字体常见）先尝试作为 2 字节大端码解码，
/// 再尝试单字节。未找到 `CMap` 时，字节按有损 UTF-8 解码
/// （传统行为）。
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

/// 解析 PDF 字符串字面量并返回原始字节（不进行有损 UTF-8 转换）。
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

/// 在位置 `pos` 解析十六进制字符串 `<XXXX>`。
///
/// 返回解码后的字节和关闭 `>` 之后的位置。
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

/// 从解压后的 PDF 内容流中提取文本（传统模式，无 `CMap`）。
///
/// 这是向后兼容的入口，委托给 [`extract_text_with_cmap`] 并传入空的
/// `CMap` 表。新代码应直接调用 [`extract_text_with_cmap`]。
#[cfg(test)]
pub(super) fn extract_text_from_content_stream(content: &[u8]) -> String {
    extract_text_with_cmap(content, &HashMap::new())
}

/// 从位置 `pos`（开头的 `(`）解析 PDF 字符串字面量。
///
/// 返回反转义后的字符串内容和关闭 `)` 之后的位置。
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

/// 从位置 `pos`（开头的 `[`）解析 TJ 数组。
///
/// 返回从数组中提取的字符串内容列表。
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
