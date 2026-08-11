//! Fuzz target: PDF to Markdown conversion with arbitrary bytes.
//!
//! Tests the full Markdown pipeline: PdfReader -> PdfDocumentModel ->
//! ProcessorPipeline -> MarkdownRenderer. Any panic or unbounded allocation
//! is a bug.
//!
//! Uses strict resource limits to prevent decompression bombs and
//! unbounded text extraction.

#![no_main]

use easypdf_core::ResourceLimits;
use easypdf_markdown::PdfMarkdownBuilder;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    // Strict limits to prevent the fuzzer from consuming too much memory.
    let limits = ResourceLimits::strict()
        .with_max_extracted_text_bytes(64 * 1024)
        .with_max_decompressed_size(256 * 1024);

    // Build a markdown conversion from arbitrary bytes.
    // This exercises: PDF parsing, text extraction, paragraph splitting,
    // heading detection, and markdown rendering.
    let builder = PdfMarkdownBuilder::from_bytes(data.to_vec())
        .resource_limits(limits);

    // do_convert may fail (invalid PDF), but must not panic.
    let _ = builder.do_convert();
});
