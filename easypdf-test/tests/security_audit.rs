//! Security audit tests: verify guards reject known attack vectors.
//!
//! This file tests three attack classes against the existing guards:
//! - A: Decompression bomb (zip-bomb-style PDFs)
//! - B: Element explosion (millions of tiny objects)
//! - C: SSRF URL validation (internal/metadata/loopback URLs)
//!
//! API key leakage (class D) is a static code review documented in
//! `docs/security/AUDIT.md` -- the OCR config types require the `ocr`
//! feature which has incompatible transitive dependencies with the
//! current Cargo.lock.
//!
//! These tests exercise the existing guards without modifying any business code.

#![allow(clippy::doc_markdown)]

use easypdf::ResourceLimits;
use easypdf::io::guards::{guard_decompression_bomb, guard_element_explosion};
use easypdf::io::ssrf_guard::validate_url;
use easypdf_ocr::baidu::BaiduConfig;
use easypdf_ocr::glm::GlmConfig;

// ==========================================================================
// A. Decompression bomb attack vectors
// ==========================================================================

/// High compression ratio on medium data: 100 KB compressed, 20 MB decompressed.
/// Ratio = 200:1, above default limit of 100:1.
#[test]
fn audit_a1_high_ratio_200_to_1() {
    let limits = ResourceLimits::default();
    let result = guard_decompression_bomb(100_000, 20_000_000, &limits);
    assert!(result.is_err(), "200:1 ratio on 100KB should be rejected");
    let msg = format!("{}", result.unwrap_err());
    assert!(
        msg.contains("compression ratio"),
        "error should mention ratio: {msg}"
    );
}

/// Extreme ratio: 1 MB compressed, 10 GB decompressed (10,000:1).
#[test]
fn audit_a2_extreme_ratio_10000_to_1() {
    let limits = ResourceLimits::default();
    let result = guard_decompression_bomb(1_000_000, 10_000_000_000, &limits);
    assert!(result.is_err(), "10,000:1 ratio should be rejected");
}

/// Nested compression simulation: each layer 10x, 3 layers = 1000:1.
/// Compressed 100 KB -> decompressed 100 MB.
#[test]
fn audit_a3_nested_compression_3_layers() {
    let limits = ResourceLimits::default();
    let result = guard_decompression_bomb(100_000, 100_000_000, &limits);
    assert!(result.is_err(), "nested 1000:1 ratio should be rejected");
}

/// Strict limits reject smaller bombs that default limits allow.
/// Strict max_compression_ratio = 50, so 60:1 on 100 KB should fail.
#[test]
fn audit_a4_strict_limits_reject_smaller_bombs() {
    let strict = ResourceLimits::strict();
    let result = guard_decompression_bomb(100_000, 6_000_000, &strict);
    assert!(
        result.is_err(),
        "60:1 ratio should be rejected under strict limits (max=50)"
    );
}

/// Boundary: exactly at the default decompressed size limit should pass.
#[test]
fn audit_a5_boundary_exact_limit_passes() {
    let limits = ResourceLimits::default();
    let max = limits.max_decompressed_size();
    // Use ratio 1:1 (compressed == decompressed) to avoid ratio check.
    let result = guard_decompression_bomb(max, max, &limits);
    assert!(result.is_ok(), "exact limit should pass");
}

/// Boundary: 1 byte over the decompressed size limit should fail.
#[test]
fn audit_a6_boundary_one_over_limit_fails() {
    let limits = ResourceLimits::default();
    let over = limits.max_decompressed_size() + 1;
    let result = guard_decompression_bomb(over, over, &limits);
    assert!(result.is_err(), "1 byte over limit should be rejected");
}

/// Small compressed data with large decompressed output: ratio check now
/// applies when decompressed >= 10 KB.  100 bytes -> 1 MB = 10,000:1
/// exceeds the 100:1 limit and is correctly rejected.
#[test]
fn audit_a7_small_compressed_large_decompressed_rejected() {
    let limits = ResourceLimits::default();
    let result = guard_decompression_bomb(100, 1_000_000, &limits);
    assert!(result.is_err(), "100B -> 1MB (10,000:1) should be rejected");
}

/// Zero compressed size: division guard, should pass.
#[test]
fn audit_a8_zero_compressed_size_no_panic() {
    let limits = ResourceLimits::default();
    let result = guard_decompression_bomb(0, 1_000_000, &limits);
    assert!(result.is_ok(), "zero compressed size should not panic");
}

/// Error code is SecurityViolation for bomb rejection.
#[test]
fn audit_a9_bomb_error_code_is_security_violation() {
    let limits = ResourceLimits::default();
    let err = guard_decompression_bomb(100_000, 100_000_000, &limits).unwrap_err();
    assert_eq!(err.code(), easypdf::PdfErrorCode::SecurityViolation);
}

/// FIXED: Small compressed payloads now trigger the ratio check when the
/// decompressed size exceeds the 10 KB safe threshold.
///
/// A 1 KB compressed payload with 1 GB decompressed (~1,000,000:1 ratio)
/// is now correctly rejected because the guard applies the ratio check
/// whenever decompressed_size >= 10 KB, regardless of compressed size.
#[test]
fn audit_a10_small_zip_bomb_ratio_now_blocked() {
    let limits = ResourceLimits::default();
    let compressed: u64 = 1_024; // 1 KB
    let decompressed: u64 = 1_000_000_000; // 1 GB (under 2 GB limit)
    let result = guard_decompression_bomb(compressed, decompressed, &limits);
    assert!(
        result.is_err(),
        "Small zip bomb (1 KB -> 1 GB, ratio ~1,000,000:1) should be rejected"
    );
    let msg = format!("{}", result.unwrap_err());
    assert!(
        msg.contains("compression ratio"),
        "error should mention ratio: {msg}"
    );
}

// ==========================================================================
// B. Element explosion attack vectors
// ==========================================================================

/// 10 million elements exceeds default limit of 5 million.
#[test]
fn audit_b1_ten_million_elements_rejected() {
    let limits = ResourceLimits::default();
    let result = guard_element_explosion(10_000_000, &limits);
    assert!(
        result.is_err(),
        "10M elements should exceed default limit of 5M"
    );
    let msg = format!("{}", result.unwrap_err());
    assert!(
        msg.contains("element count"),
        "error should mention element count: {msg}"
    );
}

/// 100,000 elements is well under default limit, should pass.
#[test]
fn audit_b2_hundred_thousand_elements_passes() {
    let limits = ResourceLimits::default();
    let result = guard_element_explosion(100_000, &limits);
    assert!(result.is_ok(), "100K elements should pass default limit");
}

/// Strict limits (1M) reject 2M elements.
#[test]
fn audit_b3_strict_limits_reject_2m_elements() {
    let strict = ResourceLimits::strict();
    let result = guard_element_explosion(2_000_000, &strict);
    assert!(
        result.is_err(),
        "2M elements should exceed strict limit of 1M"
    );
}

/// Strict limits are tighter than default on all fields.
#[test]
fn audit_b4_strict_limits_are_tighter() {
    let default = ResourceLimits::default();
    let strict = ResourceLimits::strict();
    assert!(
        strict.max_element_count() < default.max_element_count(),
        "strict max_element_count ({}) should be < default ({})",
        strict.max_element_count(),
        default.max_element_count()
    );
    assert!(
        strict.max_decompressed_size() < default.max_decompressed_size(),
        "strict max_decompressed_size should be < default"
    );
    assert!(
        strict.max_compression_ratio() < default.max_compression_ratio(),
        "strict max_compression_ratio should be < default"
    );
}

/// Boundary: exactly at the limit should pass.
#[test]
fn audit_b5_exact_element_limit_passes() {
    let limits = ResourceLimits::default();
    let result = guard_element_explosion(limits.max_element_count(), &limits);
    assert!(result.is_ok(), "exact element limit should pass");
}

/// Boundary: 1 over the limit should fail.
#[test]
fn audit_b6_one_over_element_limit_fails() {
    let limits = ResourceLimits::default();
    let result = guard_element_explosion(limits.max_element_count() + 1, &limits);
    assert!(result.is_err(), "1 over element limit should be rejected");
}

/// Error code is SecurityViolation for element explosion.
#[test]
fn audit_b7_element_error_code_is_security_violation() {
    let limits = ResourceLimits::default();
    let err = guard_element_explosion(usize::MAX, &limits).unwrap_err();
    assert_eq!(err.code(), easypdf::PdfErrorCode::SecurityViolation);
}

// ==========================================================================
// C. SSRF URL attack vectors
// ==========================================================================

/// All known SSRF attack URLs that must be rejected (IPv4 + IPv6).
#[test]
fn audit_c1_ssrf_blocked_urls() {
    let attacks = &[
        // Blocked schemes
        "file:///etc/passwd",
        "ftp://example.com/doc.pdf",
        "gopher://example.com:70/",
        "javascript:alert(1)",
        "data:text/plain,hello",
        // Blocked hostnames
        "http://localhost/admin",
        "http://localhost:8080/api",
        "http://metadata.google.internal/computeMetadata/v1/",
        "http://169.254.169.254/latest/meta-data/",
        // Loopback IPs
        "http://127.0.0.1/",
        "http://127.0.0.2/",
        "http://127.255.255.255/",
        // Private 10.x.x.x
        "http://10.0.0.1/",
        "http://10.255.255.255/",
        // Private 172.16-31.x.x
        "http://172.16.0.1/",
        "http://172.31.255.255/",
        // Private 192.168.x.x
        "http://192.168.1.1/",
        "http://192.168.0.1/",
        // Link-local 169.254.x.x
        "http://169.254.1.1/",
        "http://169.254.0.1/",
        // Zero network
        "http://0.0.0.0/",
        "http://0.0.0.1/",
        // No scheme
        "example.com/doc.pdf",
        // Empty host
        "http:///doc.pdf",
        // IPv6 loopback
        "http://[::1]/",
        "http://[::1]:8080/admin",
        // IPv6 unspecified
        "http://[::]/",
        // IPv4-mapped IPv6 loopback
        "http://[::ffff:127.0.0.1]/",
        // IPv4-mapped IPv6 private
        "http://[::ffff:10.0.0.1]/",
        // IPv4-mapped IPv6 metadata
        "http://[::ffff:169.254.169.254]/latest/meta-data/",
        // IPv6 ULA (Unique Local Address)
        "http://[fc00::1]/",
        "http://[fd00::1]/",
        // IPv6 link-local
        "http://[fe80::1]/",
        "http://[fe80::abcd:1234]/",
    ];

    for url in attacks {
        let result = validate_url(url);
        assert!(result.is_err(), "SSRF should be rejected: {url}");
        // Verify error code
        assert_eq!(
            result.unwrap_err().code(),
            easypdf::PdfErrorCode::SecurityViolation,
            "error code should be SecurityViolation for: {url}"
        );
    }
}

/// Legitimate public URLs that must be allowed.
#[test]
fn audit_c2_ssrf_allowed_urls() {
    let allowed = &[
        "https://example.com/",
        "https://example.com/doc.pdf",
        "http://example.com/doc.pdf",
        "https://cdn.example.com/files/doc.pdf?token=abc",
        "https://example.com:8443/doc.pdf",
        "https://api.openai.com/v1/ocr",
        "https://huggingface.co/models/doc.pdf",
        "http://8.8.8.8/",
        "http://1.1.1.1/",
    ];

    for url in allowed {
        let result = validate_url(url);
        assert!(result.is_ok(), "legitimate URL should be allowed: {url}");
    }
}

/// FIXED: IPv6 loopback is now blocked by the SSRF guard.
///
/// The guard now parses IPv6 addresses via `std::net::IpAddr` and
/// checks loopback, unspecified, ULA, link-local, and IPv4-mapped ranges.
#[test]
fn audit_c3_ipv6_loopback_now_blocked() {
    let result = validate_url("http://[::1]/");
    assert!(result.is_err(), "IPv6 loopback [::1] should be blocked");
    assert_eq!(
        result.unwrap_err().code(),
        easypdf::PdfErrorCode::SecurityViolation,
    );
}

/// FIXED: IPv4-mapped IPv6 loopback is now blocked.
///
/// `::ffff:127.0.0.1` is correctly identified as an IPv4-mapped loopback
/// and rejected by the SSRF guard.
#[test]
fn audit_c4_ipv4_mapped_ipv6_now_blocked() {
    let result = validate_url("http://[::ffff:127.0.0.1]/");
    assert!(
        result.is_err(),
        "IPv4-mapped loopback [::ffff:127.0.0.1] should be blocked"
    );
    assert_eq!(
        result.unwrap_err().code(),
        easypdf::PdfErrorCode::SecurityViolation,
    );
}

/// FIXED: IPv6 unspecified address is now blocked (equivalent to 0.0.0.0).
#[test]
fn audit_c5_ipv6_unspecified_now_blocked() {
    let result = validate_url("http://[::]/");
    assert!(result.is_err(), "IPv6 unspecified [::] should be blocked");
    assert_eq!(
        result.unwrap_err().code(),
        easypdf::PdfErrorCode::SecurityViolation,
    );
}

/// IPv6 ULA (Unique Local Address, fc00::/7) must be blocked.
#[test]
fn audit_c6_ipv6_ula_blocked() {
    let result = validate_url("http://[fc00::1]/");
    assert!(result.is_err(), "IPv6 ULA [fc00::1] should be blocked");
}

/// IPv6 link-local (fe80::/10) must be blocked.
#[test]
fn audit_c7_ipv6_link_local_blocked() {
    let result = validate_url("http://[fe80::1]/");
    assert!(
        result.is_err(),
        "IPv6 link-local [fe80::1] should be blocked"
    );
}

/// IPv4-mapped AWS metadata endpoint must be blocked.
#[test]
fn audit_c8_ipv4_mapped_metadata_blocked() {
    let result = validate_url("http://[::ffff:169.254.169.254]/latest/meta-data/");
    assert!(
        result.is_err(),
        "IPv4-mapped metadata endpoint should be blocked"
    );
}

// ==========================================================================
// D. API key leakage in Debug output
// ==========================================================================

/// GlmConfig must redact the api_key in Debug output.
#[test]
fn audit_d1_glm_config_redacts_api_key() {
    let config = GlmConfig {
        api_key: "super-secret-key-12345".to_string(),
        ..GlmConfig::default()
    };
    let debug_str = format!("{config:?}");
    assert!(
        !debug_str.contains("super-secret-key-12345"),
        "GlmConfig Debug must not contain raw API key"
    );
    assert!(
        debug_str.contains("redacted"),
        "GlmConfig Debug should contain 'redacted'"
    );
}

/// BaiduConfig must redact both api_key and secret_key in Debug output.
#[test]
fn audit_d2_baidu_config_redacts_both_keys() {
    let config = BaiduConfig {
        api_key: "AK-secret".to_string(),
        secret_key: "SK-extremely-secret".to_string(),
        ..BaiduConfig::default()
    };
    let debug_str = format!("{config:?}");
    assert!(
        !debug_str.contains("AK-secret"),
        "BaiduConfig Debug must not contain raw api_key"
    );
    assert!(
        !debug_str.contains("SK-extremely-secret"),
        "BaiduConfig Debug must not contain raw secret_key"
    );
    assert!(
        debug_str.contains("redacted"),
        "BaiduConfig Debug should contain 'redacted'"
    );
}
