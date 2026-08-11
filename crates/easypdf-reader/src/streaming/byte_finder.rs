//! Byte-level search utilities for raw PDF scanning.
//!
//! Provides [`find_keyword`] for locating byte sequences and [`find_endstream`]
//! for finding the `endstream` keyword with proper boundary checks.

// ---------------------------------------------------------------------------
// Byte-level search
// ---------------------------------------------------------------------------

/// Find the byte offset of `keyword` in `data` starting at `from`.
pub(super) fn find_keyword(data: &[u8], from: usize, keyword: &[u8]) -> Option<usize> {
    if keyword.is_empty() || from >= data.len() || data.len() < keyword.len() {
        return None;
    }
    let klen = keyword.len();
    let end = data.len() - klen; // safe: data.len() >= klen checked above
    (from..=end).find(|&i| data[i..i + klen] == *keyword)
}

/// Find the byte offset of `endstream` preceded by a newline and followed by
/// a non-alphanumeric character (whitespace, EOF, or a symbol), starting from
/// `from`.  Returns the offset of `endstream` itself (not the preceding `\n`).
///
/// This avoids false matches where the byte sequence `endstream` appears
/// inside binary stream data (e.g., compressed `FlateDecode` payloads).
pub(super) fn find_endstream(data: &[u8], from: usize) -> Option<usize> {
    if from >= data.len() {
        return None;
    }
    // Search for `\nendstream` (10 bytes).
    let needle = b"\nendstream";
    let nlen = needle.len();
    let end = data.len().saturating_sub(nlen);
    let search_start = from.saturating_sub(1);
    for i in search_start..=end {
        if data[i] != b'\n' {
            continue;
        }
        if i + nlen > data.len() || data[i..i + nlen] != *needle {
            continue;
        }
        let estart = i + 1;
        if estart < from {
            continue;
        }
        // Verify `endstream` is at a keyword boundary: followed by whitespace
        // or EOF.
        let after = estart + 9; // len("endstream")
        if after >= data.len() || data[after].is_ascii_whitespace() {
            return Some(estart);
        }
    }
    // Fallback: `\rendstream`.
    let needle_cr = b"\rendstream";
    let crlen = needle_cr.len();
    let end_cr = data.len().saturating_sub(crlen);
    for i in search_start..=end_cr {
        if data[i] != b'\r' {
            continue;
        }
        if i + crlen > data.len() || data[i..i + crlen] != *needle_cr {
            continue;
        }
        let estart = i + 1;
        if estart < from {
            continue;
        }
        let after = estart + 9;
        if after >= data.len() || data[after].is_ascii_whitespace() {
            return Some(estart);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Skip PDF whitespace bytes.
pub(super) fn skip_whitespace(data: &[u8], mut pos: usize) -> usize {
    while pos < data.len() && matches!(data[pos], b' ' | b'\t' | b'\n' | b'\r' | b'\x00') {
        pos += 1;
    }
    pos
}

/// Decode a single octal digit.
pub(super) fn decode_octal_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'7' => Some(b - b'0'),
        _ => None,
    }
}

/// Convert a hex ASCII byte to its 0-15 value.
pub(super) fn hex_digit_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Convert a `usize` to `u64`, saturating at `u64::MAX`.
pub(super) fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
