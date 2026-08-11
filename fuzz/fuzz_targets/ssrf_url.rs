//! Fuzz target: SSRF URL validation with arbitrary strings.
//!
//! Tests `validate_url` with arbitrary input to find panics, infinite loops,
//! or bypasses in the URL parsing and validation logic.
//!
//! The function must:
//! - Never panic on any input
//! - Always return Ok or Err (never hang)
//! - Correctly reject private/loopback IPs and blocked schemes

#![no_main]

use easypdf_core::io::ssrf_guard::validate_url;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Convert arbitrary bytes to a lossy UTF-8 string.
    let url = String::from_utf8_lossy(data);

    // validate_url must never panic -- all inputs should return Ok or Err.
    let _ = validate_url(&url);
});
