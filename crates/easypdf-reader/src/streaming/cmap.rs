//! `CMap`: character-code to Unicode mapping.
//!
//! Provides [`CMap`] for parsing PDF `ToUnicode` `CMap` streams and looking up
//! character code to Unicode codepoint mappings.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// CMap: character-code -> Unicode mapping
// ---------------------------------------------------------------------------

/// A range mapping from PDF `CMap` `beginbfrange`.
#[derive(Debug, Clone)]
pub(super) struct BfRange {
    /// First character code in the source range (inclusive).
    start_code: u32,
    /// Last character code in the source range (inclusive).
    end_code: u32,
    /// Unicode codepoint assigned to `start_code`; subsequent codes map
    /// incrementally.
    start_unicode: u32,
}

/// PDF `CMap` character-code to Unicode mapping table.
///
/// Covers the two mapping constructs used by 95 %+ of real-world PDFs:
/// - `beginbfchar` / `endbfchar` -- individual code-point mappings
/// - `beginbfrange` / `endbfrange` -- contiguous range mappings
///
/// The full `CMap` specification also includes `begin/cidchar`,
/// `begin/cidrange`, and `codespacerange`, but those are not needed for
/// `ToUnicode` extraction and are intentionally omitted for simplicity.
#[derive(Debug, Clone)]
pub(crate) struct CMap {
    /// Single-character mappings: character code -> Unicode codepoint.
    bfchar: HashMap<u32, u32>,
    /// Range mappings sorted by `start_code` (insertion order; linear scan is
    /// fine for typical `CMap` sizes of < 1 000 entries).
    bfrange: Vec<BfRange>,
}

impl CMap {
    /// Parse a `CMap` from raw stream bytes.
    ///
    /// Only `beginbfchar` and `beginbfrange` sections are extracted; all
    /// other `CMap` constructs are silently ignored.
    ///
    /// A `CMap` with zero mappings is **not** considered invalid -- it simply
    /// produces an empty table.
    #[must_use]
    pub fn parse(data: &[u8]) -> Self {
        let text = String::from_utf8_lossy(data);
        let bfchar = parse_bfchar_blocks(&text);
        let bfrange = parse_bfrange_blocks(&text);
        Self { bfchar, bfrange }
    }

    /// Look up the Unicode codepoint for `code`.
    ///
    /// `bfchar` entries are checked first (O(1) hash lookup), then
    /// `bfrange` entries are scanned linearly.  Returns `None` when no
    /// mapping exists.
    #[must_use]
    pub fn lookup(&self, code: u32) -> Option<u32> {
        // Direct bfchar mapping.
        if let Some(&unicode) = self.bfchar.get(&code) {
            return Some(unicode);
        }
        // Range mapping.
        for range in &self.bfrange {
            if code >= range.start_code && code <= range.end_code {
                let offset = code - range.start_code;
                return Some(range.start_unicode + offset);
            }
        }
        None
    }

    /// Returns `true` when the `CMap` contains zero mappings.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bfchar.is_empty() && self.bfrange.is_empty()
    }
}

// ---------------------------------------------------------------------------
// CMap parsing helpers
// ---------------------------------------------------------------------------

/// Extract all `beginbfchar ... endbfchar` blocks from `CMap` text.
fn parse_bfchar_blocks(text: &str) -> HashMap<u32, u32> {
    let mut map = HashMap::new();
    let mut remaining = text;

    while let Some(start) = remaining.find("beginbfchar") {
        let block_start = start + "beginbfchar".len();
        let Some(end) = remaining[block_start..].find("endbfchar") else {
            break;
        };
        let block = &remaining[block_start..block_start + end];
        parse_bfchar_entries(block, &mut map);
        remaining = &remaining[block_start + end + "endbfchar".len()..];
    }

    map
}

/// Parse `<src> <dst>` hex pairs inside a single `beginbfchar` block.
fn parse_bfchar_entries(block: &str, map: &mut HashMap<u32, u32>) {
    for line in block.lines() {
        let hex_tokens: Vec<&str> = line
            .split_whitespace()
            .filter(|t| t.starts_with('<') && t.ends_with('>'))
            .collect();
        for pair in hex_tokens.chunks(2) {
            if pair.len() == 2 {
                let src = parse_hex_u32(pair[0]);
                let dst = parse_hex_u32(pair[1]);
                if let (Some(s), Some(d)) = (src, dst) {
                    map.insert(s, d);
                }
            }
        }
    }
}

/// Extract all `beginbfrange ... endbfrange` blocks from `CMap` text.
fn parse_bfrange_blocks(text: &str) -> Vec<BfRange> {
    let mut ranges = Vec::new();
    let mut remaining = text;

    while let Some(start) = remaining.find("beginbfrange") {
        let block_start = start + "beginbfrange".len();
        let Some(end) = remaining[block_start..].find("endbfrange") else {
            break;
        };
        let block = &remaining[block_start..block_start + end];
        parse_bfrange_entries(block, &mut ranges);
        remaining = &remaining[block_start + end + "endbfrange".len()..];
    }

    ranges
}

/// Parse `<start> <end> <dst_start>` triplets inside a single
/// `beginbfrange` block.
fn parse_bfrange_entries(block: &str, ranges: &mut Vec<BfRange>) {
    for line in block.lines() {
        let hex_tokens: Vec<&str> = line
            .split_whitespace()
            .filter(|t| t.starts_with('<') && t.ends_with('>'))
            .collect();
        for triple in hex_tokens.chunks(3) {
            if triple.len() == 3 {
                let start_code = parse_hex_u32(triple[0]);
                let end_code = parse_hex_u32(triple[1]);
                let start_unicode = parse_hex_u32(triple[2]);
                if let (Some(sc), Some(ec), Some(su)) = (start_code, end_code, start_unicode)
                    && ec >= sc
                {
                    ranges.push(BfRange {
                        start_code: sc,
                        end_code: ec,
                        start_unicode: su,
                    });
                }
            }
        }
    }
}

/// Parse a hex string like `<0041>` into its `u32` value.
fn parse_hex_u32(token: &str) -> Option<u32> {
    let hex = token.trim_start_matches('<').trim_end_matches('>');
    u32::from_str_radix(hex, 16).ok()
}
