//! Fuzz target: streaming PDF byte scanning.
//!
//! Tests the `PdfReader` streaming strategy, which scans raw bytes for
//! `stream...endstream` boundaries, decompresses FlateDecode streams, and
//! extracts text operators -- all without building a full lopdf::Document.
//!
//! This exercises CMap parsing, hex string parsing, PDF string literal
//! parsing, decompression bomb guards, and resource limit enforcement.

#![no_main]

use easypdf_core::ResourceLimits;
use easypdf_reader::{PdfReader, ReadStrategy};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Use strict resource limits to prevent the fuzzer from allocating
    // unbounded memory on decompression bombs.
    let limits = ResourceLimits::strict()
        .with_max_extracted_text_bytes(64 * 1024)
        .with_max_decompressed_size(256 * 1024);

    let input = easypdf_core::PdfInput::from_bytes(data.to_vec());

    // Open with streaming strategy -- must never panic.
    let Ok(reader) =
        PdfReader::open_with_limits_and_strategy(&input, limits, ReadStrategy::Streaming)
    else {
        return;
    };

    // Extract text via the streaming scanner.
    let _ = reader.extract_text();

    // Extract metadata (heuristic scan).
    let _ = reader.extract_metadata();

    // Page count (heuristic).
    let _ = reader.page_count();
});
