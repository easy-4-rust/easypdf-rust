//! Fuzz target: parse arbitrary bytes as PDF via `lopdf::Document::load_mem`.
//!
//! This is the highest-priority target. `lopdf` is the core PDF parser used by
//! easypdf-reader; any panic or memory safety issue here is critical.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // lopdf::Document::load_mem must never panic on arbitrary input.
    // Errors are expected; panics are bugs.
    let _ = lopdf::Document::load_mem(data);
});
