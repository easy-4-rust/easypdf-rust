//! PDF byte-stream scanner.
//!
//! Provides [`StreamScanner`] which scans raw PDF bytes for content streams,
//! decompresses them, and extracts text operators without building a full
//! `lopdf::Document` object tree.

use std::collections::HashMap;
use std::io::Read;

use easypdf_core::ResourceLimits;
use easypdf_core::io::guards::guard_decompression_bomb;
use easypdf_core::{PdfError, PdfMetadata, PdfReadListener, Result};

use super::StreamScanResult;
use super::byte_finder::{find_endstream, find_keyword, skip_whitespace, usize_to_u64};
use super::cmap::CMap;
use super::text_extract::extract_text_with_cmap;

// ---------------------------------------------------------------------------
// Stream boundary detection types
// ---------------------------------------------------------------------------

/// A located stream region inside the raw PDF bytes.
#[derive(Debug, Clone)]
pub(super) struct StreamRange {
    /// Byte offset of the first data byte *after* the `stream` keyword and its
    /// trailing EOL (`\n` or `\r\n`).
    pub data_start: usize,
    /// Byte offset of the last data byte (exclusive -- points at the start of
    /// `endstream`).
    pub data_end: usize,
}

// ---------------------------------------------------------------------------
// StreamScanner
// ---------------------------------------------------------------------------

/// PDF byte-stream scanner.
///
/// Does **not** build a `lopdf::Document`.  Instead it directly scans the raw
/// bytes for content streams, decompresses them, and extracts text operators.
pub(crate) struct StreamScanner<'a> {
    data: &'a [u8],
    limits: ResourceLimits,
}

impl<'a> StreamScanner<'a> {
    /// Create a new scanner over the given raw PDF bytes.
    #[must_use]
    pub fn new(data: &'a [u8], limits: ResourceLimits) -> Self {
        Self { data, limits }
    }

    /// Scan all streams, extract text, and feed it to the listener.
    ///
    /// Streams are processed sequentially.  For each stream whose `/Filter`
    /// indicates `FlateDecode`, the data is decompressed before text
    /// extraction.  Streams without `FlateDecode` are scanned as-is.
    ///
    /// # Errors
    ///
    /// Returns an error when decompression fails or a resource limit is
    /// exceeded.
    pub fn scan<L: PdfReadListener + ?Sized>(&self, listener: &mut L) -> Result<StreamScanResult> {
        // Pre-scan: collect CMap tables from font ToUnicode streams.
        let cmaps = find_font_cmaps(self.data, &self.limits);

        let streams = find_streams(self.data);
        let mut result = StreamScanResult::default();
        let mut extracted_bytes = 0usize;

        for stream_range in &streams {
            let compressed = &self.data[stream_range.data_start..stream_range.data_end];

            // Inspect the dict that precedes this stream to decide on
            // decompression.
            let content = if self.has_flatedecode_filter(stream_range) {
                self.decompress_stream(compressed)?
            } else {
                compressed.to_vec()
            };

            let text = extract_text_with_cmap(&content, &cmaps);
            if !text.is_empty() {
                // We cannot map individual streams to page numbers without
                // xref, so use a synthetic page counter.
                result.pages_scanned = result.pages_scanned.saturating_add(1);
                let page_number = result.pages_scanned;
                extracted_bytes = extracted_bytes.saturating_add(text.len());
                if extracted_bytes > self.limits.max_extracted_text_bytes() {
                    return Err(PdfError::ResourceLimitExceeded {
                        resource: "extracted_text_bytes",
                        limit: usize_to_u64(self.limits.max_extracted_text_bytes()),
                        actual: usize_to_u64(extracted_bytes),
                    });
                }
                listener.on_page_start(page_number)?;
                listener.on_text(page_number, &text)?;
                listener.on_page_end(page_number)?;
                result.text_extracted = true;
            }
            result.streams_processed = result.streams_processed.saturating_add(1);
        }

        listener.on_document_end()?;
        Ok(result)
    }

    /// Heuristic page count by counting `/Type /Page` entries.
    ///
    /// This scans the raw bytes for the pattern `/Type/Page` or
    /// `/Type /Page` (with optional whitespace).  It is a fast approximation
    /// and may over-count if the pattern appears inside stream data or
    /// strings.
    #[must_use]
    pub fn page_count(&self) -> usize {
        count_page_entries(self.data)
    }

    /// Extract metadata by scanning for `/Info` dictionary keys.
    ///
    /// This is a best-effort scan of the raw bytes and may return partial
    /// or empty metadata for encrypted or unusual PDFs.
    #[must_use]
    pub fn extract_metadata_quick(&self) -> PdfMetadata {
        extract_metadata_from_bytes(self.data)
    }

    /// Decompress a `FlateDecode` stream with bomb protection.
    pub(super) fn decompress_stream(&self, compressed: &[u8]) -> Result<Vec<u8>> {
        // Pre-check: guard against decompression bomb using compressed size.
        // We pass 0 for decompressed size because we don't know it yet.
        guard_decompression_bomb(compressed.len() as u64, 0, &self.limits)?;

        let mut decoder = flate2::read::ZlibDecoder::new(compressed);
        let mut output = Vec::new();
        decoder
            .read_to_end(&mut output)
            .map_err(|e| PdfError::Parse(format!("FlateDecode decompression failed: {e}")))?;

        // Post-check: guard with actual decompressed size.
        guard_decompression_bomb(compressed.len() as u64, output.len() as u64, &self.limits)?;

        Ok(output)
    }

    /// Check whether the stream at `range` has `/Filter /FlateDecode` in its
    /// preceding dictionary.
    pub(super) fn has_flatedecode_filter(&self, range: &StreamRange) -> bool {
        // Scan backwards from `data_start` (which is right after "stream\n")
        // to find the enclosing `<< ... >>` dict.  Look for `/Filter`.
        // We limit the look-back to 4 KB to avoid scanning huge dicts.
        let look_back = range.data_start.min(4096);
        let dict_region = &self.data[range.data_start - look_back..range.data_start];

        // Look for /Filter followed by /FlateDecode (with optional whitespace).
        let region_str = String::from_utf8_lossy(dict_region);
        if let Some(filter_pos) = region_str.find("/Filter") {
            let after_filter = &region_str[filter_pos + 7..];
            // Skip whitespace and optional array `[`
            let trimmed = after_filter.trim_start();
            return trimmed.starts_with("/FlateDecode")
                || trimmed.starts_with("[/FlateDecode")
                || trimmed.starts_with("[ /FlateDecode");
        }
        false
    }
}

// ---------------------------------------------------------------------------
// Stream boundary detection
// ---------------------------------------------------------------------------

/// Find all `stream...endstream` boundaries in the raw PDF bytes.
fn find_streams(data: &[u8]) -> Vec<StreamRange> {
    let mut streams = Vec::new();
    let mut pos = 0;

    while pos < data.len() {
        // Find next occurrence of "stream" keyword.
        let Some(stream_start) = find_keyword(data, pos, b"stream") else {
            break;
        };

        // The data starts after "stream" + EOL.
        let after_keyword = stream_start + 6; // len("stream")
        if after_keyword >= data.len() {
            break;
        }

        let data_start = match data[after_keyword] {
            b'\n' => after_keyword + 1,
            b'\r' => {
                if after_keyword + 1 < data.len() && data[after_keyword + 1] == b'\n' {
                    after_keyword + 2
                } else {
                    after_keyword + 1
                }
            }
            _ => {
                // Not a valid stream keyword (no EOL after it).
                pos = after_keyword;
                continue;
            }
        };

        // Find "endstream" after data_start.  We search for `\nendstream`
        // rather than bare `endstream` to avoid false matches inside binary
        // stream data (e.g., compressed FlateDecode payloads).
        if let Some(endstream_start) = find_endstream(data, data_start) {
            // The PDF spec requires an EOL before "endstream".  Trim exactly
            // one trailing EOL (`\r\n` or `\n` or `\r`) but nothing more --
            // trimming arbitrary whitespace could eat into binary stream data.
            let mut data_end = endstream_start;
            if data_end > data_start {
                // Trim exactly one trailing EOL.
                if data[data_end - 1] == b'\n' {
                    data_end -= 1;
                    if data_end > data_start && data[data_end - 1] == b'\r' {
                        data_end -= 1;
                    }
                } else if data[data_end - 1] == b'\r' {
                    data_end -= 1;
                }
            }

            if data_end > data_start {
                streams.push(StreamRange {
                    data_start,
                    data_end,
                });
            }

            pos = endstream_start + 9; // len("endstream")
        } else {
            // No matching endstream -- skip.
            pos = after_keyword;
        }
    }

    streams
}

// ---------------------------------------------------------------------------
// Font CMap discovery
// ---------------------------------------------------------------------------

/// Scan raw PDF bytes for font objects that carry a `/ToUnicode` `CMap` stream.
///
/// Returns a map from the font's base name (e.g. `"F1"`) to its parsed
/// [`CMap`].  Font names are extracted from the `/Name` key of the font
/// dictionary, falling back to the object number as a synthetic name
/// (e.g. `"obj_5"`).
///
/// `ToUnicode` streams are decompressed inline (`FlateDecode` only) and
/// parsed into [`CMap`] tables.  Errors during decompression or parsing of
/// individual fonts are silently skipped so that a single corrupt font does
/// not prevent the rest of the document from being processed.
fn find_font_cmaps(data: &[u8], limits: &ResourceLimits) -> HashMap<String, CMap> {
    let mut cmaps = HashMap::new();

    // Step 1: find all font dictionary regions by looking for /Type /Font.
    // We scan byte-by-byte for the pattern and then extract the surrounding
    // dict region to find /ToUnicode and /BaseFont or /Name.
    let text = String::from_utf8_lossy(data);
    let mut search_from = 0;

    while let Some(type_pos) = find_subsequence_from(&text, search_from, "/Type") {
        let after_type = type_pos + 5; // len("/Type")
        // Skip whitespace.
        let ws_end = after_type
            + text[after_type..]
                .bytes()
                .take_while(u8::is_ascii_whitespace)
                .count();
        if !text[ws_end..].starts_with("/Font") {
            search_from = after_type;
            continue;
        }
        // We have /Type /Font at type_pos.  Extract a surrounding region for
        // the dict.  Scan backwards for `<<` and forwards for `>>`.
        let dict_start = text[..type_pos].rfind("<<").unwrap_or(0);
        let font_region_end = text[ws_end + 5..]
            .find(">>")
            .map_or(text.len(), |p| ws_end + 5 + p + 2);
        let font_region = &text[dict_start..font_region_end];

        // Extract font name: /BaseFont or a synthetic name from object number.
        let font_name = extract_font_name(font_region, type_pos - dict_start);

        // Extract /ToUnicode indirect reference (e.g. "12 0 R").
        if let Some(tu_ref) = extract_to_unicode_ref(font_region) {
            // Resolve the indirect reference to the actual stream data.
            if let Some(cmap) = resolve_to_unicode_cmap(data, tu_ref, limits) {
                cmaps.insert(font_name, cmap);
            }
        }

        search_from = font_region_end;
    }

    cmaps
}

/// Find the first occurrence of `needle` in `haystack` starting at `from`.
fn find_subsequence_from(haystack: &str, from: usize, needle: &str) -> Option<usize> {
    if from >= haystack.len() {
        return None;
    }
    haystack[from..].find(needle).map(|p| p + from)
}

/// Extract a human-readable font name from a font dictionary region.
///
/// Prefers `/BaseFont`, then `/Name`, then synthesises from the surrounding
/// object number.
fn extract_font_name(region: &str, offset: usize) -> String {
    // Try /BaseFont /SomeName
    if let Some(pos) = region.find("/BaseFont") {
        let after = &region[pos + 9..].trim_start();
        if let Some(name) = extract_pdf_name(after) {
            return name.to_owned();
        }
    }
    // Try /Name (some CJK fonts use this)
    if let Some(pos) = region.find("/Name") {
        let after = &region[pos + 5..].trim_start();
        if let Some(name) = extract_pdf_name(after) {
            return name.to_owned();
        }
    }
    // Fallback: use byte offset as synthetic key.
    format!("font_{}", offset)
}

/// Extract a PDF name token (e.g. `/F1`) from the start of `s`.
fn extract_pdf_name(s: &str) -> Option<&str> {
    if !s.starts_with('/') {
        return None;
    }
    let end = s[1..]
        .find(|c: char| c.is_ascii_whitespace() || c == '/' || c == '>' || c == '[')
        .map_or(s.len(), |p| p + 1);
    if end > 1 {
        Some(&s[1..end]) // without leading '/'
    } else {
        None
    }
}

/// Extract the indirect object reference after `/ToUnicode` in a font region.
///
/// Returns the (`object_number`, `generation_number`) pair, e.g. `(12, 0)`
/// for `/ToUnicode 12 0 R`.
fn extract_to_unicode_ref(region: &str) -> Option<(u32, u32)> {
    let pos = region.find("/ToUnicode")?;
    let after = region[pos + 10..].trim_start();
    let mut parts = after.split_whitespace();
    let obj_num: u32 = parts.next()?.parse().ok()?;
    let gen_num: u32 = parts.next()?.parse().ok()?;
    // The next token should be "R" but we don't strictly enforce it.
    Some((obj_num, gen_num))
}

/// Resolve an indirect object reference to its stream bytes and parse as `CMap`.
///
/// Scans the raw PDF for the object definition (`N G obj ... stream ... endstream`)
/// and, if present, decompresses and parses the `CMap`.
fn resolve_to_unicode_cmap(
    data: &[u8],
    (obj_num, gen_num): (u32, u32),
    limits: &ResourceLimits,
) -> Option<CMap> {
    // Find "N G obj" pattern.
    let obj_header = format!("{obj_num} {gen_num} obj");
    let obj_pos = find_keyword(data, 0, obj_header.as_bytes())?;
    let region = &data[obj_pos..];

    // Find the stream within this object.
    let stream_pos = find_keyword(region, 0, b"stream")?;
    let after_stream = stream_pos + 6; // len("stream")
    if after_stream >= region.len() {
        return None;
    }

    let data_start = match region[after_stream] {
        b'\n' => after_stream + 1,
        b'\r' => {
            if after_stream + 1 < region.len() && region[after_stream + 1] == b'\n' {
                after_stream + 2
            } else {
                after_stream + 1
            }
        }
        _ => return None,
    };

    // Find endstream.
    let endstream_pos = find_endstream(region, data_start)?;
    let mut data_end = endstream_pos;
    if data_end > data_start {
        if region[data_end - 1] == b'\n' {
            data_end -= 1;
            if data_end > data_start && region[data_end - 1] == b'\r' {
                data_end -= 1;
            }
        } else if region[data_end - 1] == b'\r' {
            data_end -= 1;
        }
    }
    if data_end <= data_start {
        return None;
    }

    let raw_stream = &region[data_start..data_end];

    // Decompress if FlateDecode is indicated in the preceding dict.
    let dict_region = &region[..data_start];
    let dict_text = String::from_utf8_lossy(dict_region);
    let cmap_bytes = if dict_text.contains("/FlateDecode") {
        decompress_stream_inline(raw_stream, limits).ok()?
    } else {
        raw_stream.to_vec()
    };

    Some(CMap::parse(&cmap_bytes))
}

/// Inline decompression of a `FlateDecode` stream (no `&self` needed).
fn decompress_stream_inline(compressed: &[u8], limits: &ResourceLimits) -> Result<Vec<u8>> {
    guard_decompression_bomb(compressed.len() as u64, 0, limits)?;

    let mut decoder = flate2::read::ZlibDecoder::new(compressed);
    let mut output = Vec::new();
    decoder
        .read_to_end(&mut output)
        .map_err(|e| PdfError::Parse(format!("ToUnicode FlateDecode decompression failed: {e}")))?;

    guard_decompression_bomb(compressed.len() as u64, output.len() as u64, limits)?;

    Ok(output)
}

// ---------------------------------------------------------------------------
// Metadata extraction (heuristic, no xref)
// ---------------------------------------------------------------------------

/// Count `/Type /Page` (or `/Type/Page`) entries in the raw bytes.
fn count_page_entries(data: &[u8]) -> usize {
    let needle = b"/Type";
    let mut count = 0;
    let mut pos = 0;

    while pos < data.len() {
        if let Some(offset) = find_keyword(data, pos, needle) {
            let after = offset + 5; // len("/Type")
            let rest = skip_whitespace(data, after);
            if rest < data.len() && data[rest..].starts_with(b"/Page") {
                // Ensure it's exactly "/Page" and not "/Pages".
                let page_end = rest + 5; // len("/Page")
                if page_end >= data.len() || !data[page_end].is_ascii_alphabetic() {
                    count += 1;
                }
            }
            pos = after;
        } else {
            break;
        }
    }

    count
}

/// Extract metadata from raw PDF bytes by scanning for `/Title`, `/Author`, etc.
fn extract_metadata_from_bytes(data: &[u8]) -> PdfMetadata {
    let mut metadata = PdfMetadata::new();

    metadata.title = extract_info_string(data, b"/Title");
    metadata.author = extract_info_string(data, b"/Author");
    metadata.subject = extract_info_string(data, b"/Subject");
    metadata.keywords = extract_info_string(data, b"/Keywords");
    metadata.creator = extract_info_string(data, b"/Creator");
    metadata.producer = extract_info_string(data, b"/Producer");

    metadata
}

/// Find a key like `/Title` and extract the following PDF string `(...)`.
fn extract_info_string(data: &[u8], key: &[u8]) -> Option<String> {
    use super::text_extract::parse_pdf_string;

    let pos = find_keyword(data, 0, key)?;
    let after_key = pos + key.len();
    let rest = skip_whitespace(data, after_key);

    if rest < data.len() && data[rest] == b'(' {
        let (s, _) = parse_pdf_string(data, rest)?;
        if s.is_empty() {
            return None;
        }
        return Some(s);
    }

    // The value might be an indirect reference (e.g., "5 0 R") -- skip those
    // as we don't resolve indirect refs in streaming mode.
    None
}

// ---------------------------------------------------------------------------
// Test helpers (expose private functions for sibling test module)
// ---------------------------------------------------------------------------

#[cfg(test)]
pub(super) fn find_streams_for_test(data: &[u8]) -> Vec<StreamRange> {
    find_streams(data)
}

#[cfg(test)]
pub(super) fn extract_font_name_for_test(region: &str, offset: usize) -> String {
    extract_font_name(region, offset)
}

#[cfg(test)]
pub(super) fn extract_to_unicode_ref_for_test(region: &str) -> Option<(u32, u32)> {
    extract_to_unicode_ref(region)
}
