//! 用于原始 PDF 扫描的字节级搜索工具。
//!
//! 提供 [`find_keyword`] 用于定位字节序列，[`find_endstream`] 用于
//! 在正确的边界检查下查找 `endstream` 关键字。

// ---------------------------------------------------------------------------
// Byte-level search
// ---------------------------------------------------------------------------

/// 在 `data` 中从 `from` 开始查找 `keyword` 的字节偏移量。
pub(super) fn find_keyword(data: &[u8], from: usize, keyword: &[u8]) -> Option<usize> {
    if keyword.is_empty() || from >= data.len() || data.len() < keyword.len() {
        return None;
    }
    let klen = keyword.len();
    let end = data.len() - klen; // safe: data.len() >= klen checked above
    (from..=end).find(|&i| data[i..i + klen] == *keyword)
}

/// 从 `from` 开始查找以换行符开头、后跟非字母数字字符（空白、EOF 或符号）
/// 的 `endstream` 的字节偏移量。返回 `endstream` 本身的偏移量（不是前面的 `\n`）。
///
/// 这避免了字节序列 `endstream` 出现在二进制流数据
/// （如压缩的 `FlateDecode` 载荷）中的误匹配。
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

/// 跳过 PDF 空白字节。
pub(super) fn skip_whitespace(data: &[u8], mut pos: usize) -> usize {
    while pos < data.len() && matches!(data[pos], b' ' | b'\t' | b'\n' | b'\r' | b'\x00') {
        pos += 1;
    }
    pos
}

/// 解码单个八进制数字。
pub(super) fn decode_octal_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'7' => Some(b - b'0'),
        _ => None,
    }
}

/// 将十六进制 ASCII 字节转换为其 0-15 的值。
pub(super) fn hex_digit_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// 将 `usize` 转换为 `u64`，溢出时饱和到 `u64::MAX`。
pub(super) fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
