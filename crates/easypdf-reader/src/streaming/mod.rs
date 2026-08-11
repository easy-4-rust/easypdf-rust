//! Streaming PDF byte-stream scanner.
//!
//! Scans raw PDF bytes for `stream...endstream` boundaries, decompresses
//! content streams, and extracts text operators without building a full
//! `lopdf::Document` object tree.  Designed for very large PDFs (>100 MB)
//! or resource-constrained environments where the overhead of a complete
//! xref/object-tree parse is unacceptable.

mod byte_finder;
mod cmap;
pub(super) mod scanner;
mod text_extract;

#[cfg(test)]
mod tests;

// Re-export the public types at the streaming module level so that the
// parent (`lib.rs`) can use `streaming::StreamScanner` and
// `streaming::StreamScanResult`.
pub(super) use scanner::StreamScanner;

/// Result of a streaming scan pass.
#[derive(Debug, Clone, Default)]
pub(crate) struct StreamScanResult {
    /// Number of pages detected (heuristic: `/Type /Page` entries).
    pub pages_scanned: usize,
    /// Number of stream objects that were processed.
    pub streams_processed: usize,
    /// Whether any text was extracted.
    pub text_extracted: bool,
}
