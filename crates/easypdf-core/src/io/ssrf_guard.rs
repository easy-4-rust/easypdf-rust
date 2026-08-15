//! SSRF (Server-Side Request Forgery) guard for URL validation.
//!
//! When a [`PdfInput`](crate::PdfInput) accepts URLs as a source, this
//! module provides validation to prevent requests to internal or
//! sensitive network endpoints.
//!
//! **Status**: The current [`PdfInput`](crate::PdfInput) only supports
//! local file paths and in-memory bytes.  The URL validation functions
//! are provided as a public API for future use and for downstream
//! crates that may add URL-based input sources.

use crate::{PdfError, Result};

/// URL schemes that are allowed for remote PDF fetching.
///
/// Only `http` and `https` are considered safe.  All other schemes
/// (`file://`, `ftp://`, `data:`, etc.) are rejected.
const ALLOWED_SCHEMES: &[&str] = &["http", "https"];

/// Private / loopback IP prefixes (IPv4) that must be rejected.
///
/// This covers:
/// - `127.0.0.0/8` (loopback)
/// - `10.0.0.0/8` (private)
/// - `172.16.0.0/12` (private)
/// - `192.168.0.0/16` (private)
/// - `169.254.0.0/16` (link-local / cloud metadata)
/// - `0.0.0.0/8` (this network)
const BLOCKED_HOSTS: &[&str] = &[
    "localhost",
    "0.0.0.0",
    "127.0.0.1",
    "metadata.google.internal",
    "169.254.169.254",
];

/// Validate a URL, rejecting schemes and hosts that could be used for
/// SSRF attacks.
///
/// # Rejection rules
///
/// - Scheme must be `http` or `https`.
/// - Host must not be empty.
/// - Host must not be a known private/loopback/metadata hostname.
/// - Host must not parse as a private/loopback IPv4 address.
///
/// # Errors
///
/// Returns [`PdfError::SecurityViolation`] when the URL fails any check.
///
/// # Examples
///
/// ```
/// use easypdf_core::io::ssrf_guard::validate_url;
///
/// assert!(validate_url("https://example.com/doc.pdf").is_ok());
/// assert!(validate_url("http://example.com/doc.pdf").is_ok());
///
/// // Blocked schemes.
/// assert!(validate_url("file:///etc/passwd").is_err());
/// assert!(validate_url("ftp://example.com/doc.pdf").is_err());
///
/// // Blocked hosts.
/// assert!(validate_url("http://localhost/doc.pdf").is_err());
/// assert!(validate_url("http://169.254.169.254/latest/meta-data").is_err());
/// ```
pub fn validate_url(url: &str) -> Result<()> {
    // Split scheme.
    let scheme_end = url
        .find(':')
        .ok_or_else(|| PdfError::SecurityViolation(format!("URL has no scheme: {url}")))?;
    let scheme = &url[..scheme_end].to_ascii_lowercase();

    if !ALLOWED_SCHEMES.contains(&scheme.as_str()) {
        return Err(PdfError::SecurityViolation(format!(
            "URL scheme '{scheme}' is not allowed (only http/https)"
        )));
    }

    // Extract host from `://host[:port][/...]`.
    let after_scheme = &url[scheme_end + 1..];
    let authority = after_scheme
        .strip_prefix("//")
        .ok_or_else(|| PdfError::SecurityViolation(format!("URL has no authority: {url}")))?;

    // Host is up to the first `/`, `?`, or `#`.
    let host_end = authority.find(['/', '?', '#']).unwrap_or(authority.len());
    let host_with_port = &authority[..host_end];

    // Strip port if present.
    let host = if host_with_port.starts_with('[') {
        // IPv6 -- extract bracketed address.
        host_with_port
            .find(']')
            .map_or(host_with_port, |end| &host_with_port[1..end])
    } else {
        host_with_port
            .rsplit_once(':')
            .map_or(host_with_port, |(h, _)| h)
    };

    if host.is_empty() {
        return Err(PdfError::SecurityViolation(
            "URL has empty host".to_string(),
        ));
    }

    let host_lower = host.to_ascii_lowercase();

    // Check blocked hostnames.
    for blocked in BLOCKED_HOSTS {
        if host_lower == *blocked {
            return Err(PdfError::SecurityViolation(format!(
                "host '{host}' is blocked (private/loopback/metadata)"
            )));
        }
    }

    // Check IP addresses (IPv4 and IPv6) via std::net parser.
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        match ip {
            std::net::IpAddr::V4(v4) => {
                if is_private_ipv4_addr(v4) {
                    return Err(PdfError::SecurityViolation(format!(
                        "host '{host}' is a private/loopback IPv4 address"
                    )));
                }
            }
            std::net::IpAddr::V6(v6) => {
                if is_blocked_ipv6(v6) {
                    return Err(PdfError::SecurityViolation(format!(
                        "host '{host}' is a blocked IPv6 address"
                    )));
                }
            }
        }
    } else if is_private_ipv4(host) {
        // Fallback for IPv4 addresses that don't parse via std::net
        // (shouldn't happen, but preserves existing behavior).
        return Err(PdfError::SecurityViolation(format!(
            "host '{host}' is a private/loopback IPv4 address"
        )));
    }

    Ok(())
}

/// Check if a string looks like a private/loopback IPv4 address.
///
/// Returns `false` for hostnames, IPv6, or unparseable strings.
fn is_private_ipv4(host: &str) -> bool {
    let parts: Vec<u8> = host
        .split('.')
        .map(str::parse::<u8>)
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap_or_default();

    if parts.len() != 4 {
        return false;
    }

    let [a, b, _c, _d] = [parts[0], parts[1], parts[2], parts[3]];

    // 127.0.0.0/8 (loopback)
    if a == 127 {
        return true;
    }
    // 10.0.0.0/8 (private)
    if a == 10 {
        return true;
    }
    // 172.16.0.0/12 (private)
    if a == 172 && (16..=31).contains(&b) {
        return true;
    }
    // 192.168.0.0/16 (private)
    if a == 192 && b == 168 {
        return true;
    }
    // 169.254.0.0/16 (link-local / cloud metadata)
    if a == 169 && b == 254 {
        return true;
    }
    // 0.0.0.0/8 (this network)
    if a == 0 {
        return true;
    }

    false
}

/// Check if a parsed [`std::net::Ipv4Addr`] is in a private/loopback range.
fn is_private_ipv4_addr(addr: std::net::Ipv4Addr) -> bool {
    addr.is_loopback()
        || addr.is_private()
        || addr.is_link_local()
        || addr.is_unspecified()
        || addr.octets()[0] == 0 // 0.0.0.0/8 (this network)
}

/// Check if a parsed [`std::net::Ipv6Addr`] is in a blocked range.
///
/// Blocks: loopback (`::1`), unspecified (`::`), ULA (`fc00::/7`),
/// link-local (`fe80::/10`), and IPv4-mapped addresses that embed a
/// private IPv4.
fn is_blocked_ipv6(addr: std::net::Ipv6Addr) -> bool {
    // ::1 -- loopback
    if addr.is_loopback() {
        return true;
    }
    // :: -- unspecified
    if addr.is_unspecified() {
        return true;
    }
    let segments = addr.segments();
    // fc00::/7 -- Unique Local Address (ULA)
    if segments[0] & 0xfe00 == 0xfc00 {
        return true;
    }
    // fe80::/10 -- link-local
    if segments[0] & 0xffc0 == 0xfe80 {
        return true;
    }
    // IPv4-mapped IPv6 (::ffff:x.x.x.x) -- check the embedded IPv4.
    if let Some(v4) = addr.to_ipv4_mapped() {
        return is_private_ipv4_addr(v4);
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Valid URLs ---

    #[test]
    fn https_public_host_passes() {
        assert!(validate_url("https://example.com/doc.pdf").is_ok());
    }

    #[test]
    fn http_public_host_passes() {
        assert!(validate_url("http://example.com/doc.pdf").is_ok());
    }

    #[test]
    fn https_with_port_passes() {
        assert!(validate_url("https://example.com:8443/doc.pdf").is_ok());
    }

    #[test]
    fn https_with_path_and_query_passes() {
        assert!(validate_url("https://cdn.example.com/files/doc.pdf?token=abc").is_ok());
    }

    // --- Blocked schemes ---

    #[test]
    fn file_scheme_rejected() {
        let result = validate_url("file:///etc/passwd");
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("not allowed"));
    }

    #[test]
    fn ftp_scheme_rejected() {
        assert!(validate_url("ftp://example.com/doc.pdf").is_err());
    }

    #[test]
    fn data_scheme_rejected() {
        assert!(validate_url("data:text/plain,hello").is_err());
    }

    // --- Blocked hosts ---

    #[test]
    fn localhost_rejected() {
        assert!(validate_url("http://localhost/doc.pdf").is_err());
    }

    #[test]
    fn localhost_with_port_rejected() {
        assert!(validate_url("http://localhost:8080/doc.pdf").is_err());
    }

    #[test]
    fn metadata_endpoint_rejected() {
        assert!(validate_url("http://169.254.169.254/latest/meta-data").is_err());
    }

    #[test]
    fn metadata_hostname_rejected() {
        assert!(validate_url("http://metadata.google.internal/computeMetadata/v1/").is_err());
    }

    // --- Private IP ranges ---

    #[test]
    fn loopback_127_rejected() {
        assert!(validate_url("http://127.0.0.1/doc.pdf").is_err());
        assert!(validate_url("http://127.0.0.2/doc.pdf").is_err());
    }

    #[test]
    fn private_10_rejected() {
        assert!(validate_url("http://10.0.0.1/doc.pdf").is_err());
        assert!(validate_url("http://10.255.255.255/doc.pdf").is_err());
    }

    #[test]
    fn private_172_16_rejected() {
        assert!(validate_url("http://172.16.0.1/doc.pdf").is_err());
        assert!(validate_url("http://172.31.255.255/doc.pdf").is_err());
    }

    #[test]
    fn private_192_168_rejected() {
        assert!(validate_url("http://192.168.1.1/doc.pdf").is_err());
    }

    #[test]
    fn link_local_169_254_rejected() {
        assert!(validate_url("http://169.254.1.1/doc.pdf").is_err());
    }

    #[test]
    fn zero_network_rejected() {
        assert!(validate_url("http://0.0.0.0/doc.pdf").is_err());
    }

    // --- Edge cases ---

    #[test]
    fn no_scheme_rejected() {
        assert!(validate_url("example.com/doc.pdf").is_err());
    }

    #[test]
    fn empty_host_rejected() {
        assert!(validate_url("http:///doc.pdf").is_err());
    }

    #[test]
    fn public_ip_passes() {
        assert!(validate_url("http://8.8.8.8/doc.pdf").is_ok());
        assert!(validate_url("http://1.1.1.1/doc.pdf").is_ok());
    }

    #[test]
    fn security_violation_error_code() {
        let err = validate_url("file:///etc/passwd").unwrap_err();
        assert_eq!(err.code(), crate::PdfErrorCode::SecurityViolation);
    }

    // --- IPv6 blocked ---

    #[test]
    fn ipv6_loopback_rejected() {
        assert!(validate_url("http://[::1]/").is_err());
    }

    #[test]
    fn ipv6_unspecified_rejected() {
        assert!(validate_url("http://[::]/").is_err());
    }

    #[test]
    fn ipv4_mapped_loopback_rejected() {
        assert!(validate_url("http://[::ffff:127.0.0.1]/").is_err());
    }

    #[test]
    fn ipv4_mapped_private_rejected() {
        assert!(validate_url("http://[::ffff:10.0.0.1]/").is_err());
    }

    #[test]
    fn ipv4_mapped_metadata_rejected() {
        assert!(validate_url("http://[::ffff:169.254.169.254]/latest/meta-data/").is_err());
    }

    #[test]
    fn ipv6_ula_rejected() {
        assert!(validate_url("http://[fc00::1]/").is_err());
    }

    #[test]
    fn ipv6_link_local_rejected() {
        assert!(validate_url("http://[fe80::1]/").is_err());
    }

    #[test]
    fn ipv6_global_passes() {
        // A public IPv6 address should be allowed.
        assert!(validate_url("http://[2606:4700:4700::1111]/").is_ok());
    }

    // --- Additional edge cases for coverage ---

    #[test]
    fn url_with_fragment_passes() {
        assert!(validate_url("https://example.com/doc.pdf#page=1").is_ok());
    }

    #[test]
    fn url_with_query_only_passes() {
        assert!(validate_url("https://example.com?token=abc").is_ok());
    }

    #[test]
    fn ipv6_with_port_passes() {
        assert!(validate_url("http://[2606:4700:4700::1111]:8080/").is_ok());
    }

    #[test]
    fn ipv6_with_port_and_path_passes() {
        assert!(validate_url("http://[2606:4700:4700::1111]:8080/doc.pdf").is_ok());
    }

    #[test]
    fn no_authority_rejected() {
        // Missing "//" after scheme
        assert!(validate_url("http:example.com/doc.pdf").is_err());
    }

    #[test]
    fn scheme_only_rejected() {
        assert!(validate_url("http://").is_err());
    }

    #[test]
    fn uppercase_scheme_passes() {
        assert!(validate_url("HTTPS://example.com/doc.pdf").is_ok());
    }

    #[test]
    fn uppercase_blocked_host_rejected() {
        assert!(validate_url("http://LOCALHOST/doc.pdf").is_err());
    }

    #[test]
    fn mixed_case_localhost_rejected() {
        assert!(validate_url("http://LocalHost/doc.pdf").is_err());
    }

    #[test]
    fn private_0_prefix_rejected() {
        assert!(validate_url("http://0.1.2.3/doc.pdf").is_err());
    }

    #[test]
    fn is_private_ipv4_non_numeric_rejected() {
        // is_private_ipv4 returns false for non-IPv4 strings
        assert!(!is_private_ipv4("not-an-ip"));
        assert!(!is_private_ipv4("abc.def.ghi.jkl"));
    }

    #[test]
    fn is_private_ipv4_three_octets_rejected() {
        // Only 4 parts should be considered
        assert!(!is_private_ipv4("10.0.0"));
    }

    #[test]
    fn is_private_ipv4_five_octets_rejected() {
        assert!(!is_private_ipv4("10.0.0.0.0"));
    }

    #[test]
    fn is_private_ipv4_loopback_range() {
        assert!(is_private_ipv4("127.0.0.1"));
        assert!(is_private_ipv4("127.255.255.255"));
    }

    #[test]
    fn is_private_ipv4_private_10() {
        assert!(is_private_ipv4("10.0.0.1"));
        assert!(is_private_ipv4("10.255.255.255"));
    }

    #[test]
    fn is_private_ipv4_private_172() {
        assert!(is_private_ipv4("172.16.0.1"));
        assert!(is_private_ipv4("172.31.255.255"));
        assert!(!is_private_ipv4("172.32.0.1"));
        assert!(!is_private_ipv4("172.15.0.1"));
    }

    #[test]
    fn is_private_ipv4_private_192_168() {
        assert!(is_private_ipv4("192.168.0.1"));
        assert!(is_private_ipv4("192.168.255.255"));
    }

    #[test]
    fn is_private_ipv4_link_local() {
        assert!(is_private_ipv4("169.254.1.1"));
        assert!(is_private_ipv4("169.254.255.255"));
    }

    #[test]
    fn is_private_ipv4_zero_network() {
        assert!(is_private_ipv4("0.0.0.0"));
        assert!(is_private_ipv4("0.255.255.255"));
    }

    #[test]
    fn is_private_ipv4_public_passes() {
        assert!(!is_private_ipv4("8.8.8.8"));
        assert!(!is_private_ipv4("1.1.1.1"));
        assert!(!is_private_ipv4("203.0.113.1"));
    }

    #[test]
    fn is_blocked_ipv6_loopback() {
        let addr: std::net::Ipv6Addr = "::1".parse().unwrap();
        assert!(is_blocked_ipv6(addr));
    }

    #[test]
    fn is_blocked_ipv6_unspecified() {
        let addr: std::net::Ipv6Addr = "::".parse().unwrap();
        assert!(is_blocked_ipv6(addr));
    }

    #[test]
    fn is_blocked_ipv6_ula() {
        let addr: std::net::Ipv6Addr = "fc00::1".parse().unwrap();
        assert!(is_blocked_ipv6(addr));
        let addr2: std::net::Ipv6Addr = "fd00::1".parse().unwrap();
        assert!(is_blocked_ipv6(addr2));
    }

    #[test]
    fn is_blocked_ipv6_link_local() {
        let addr: std::net::Ipv6Addr = "fe80::1".parse().unwrap();
        assert!(is_blocked_ipv6(addr));
    }

    #[test]
    fn is_blocked_ipv6_global_passes() {
        let addr: std::net::Ipv6Addr = "2606:4700:4700::1111".parse().unwrap();
        assert!(!is_blocked_ipv6(addr));
    }

    #[test]
    fn is_blocked_ipv6_ipv4_mapped_private() {
        let addr: std::net::Ipv6Addr = "::ffff:10.0.0.1".parse().unwrap();
        assert!(is_blocked_ipv6(addr));
    }

    #[test]
    fn is_blocked_ipv6_ipv4_mapped_public_passes() {
        let addr: std::net::Ipv6Addr = "::ffff:8.8.8.8".parse().unwrap();
        assert!(!is_blocked_ipv6(addr));
    }

    #[test]
    fn private_172_outside_range_passes() {
        assert!(validate_url("http://172.32.0.1/doc.pdf").is_ok());
        assert!(validate_url("http://172.15.0.1/doc.pdf").is_ok());
    }

    #[test]
    fn private_192_not_168_passes() {
        assert!(validate_url("http://192.169.0.1/doc.pdf").is_ok());
    }

    #[test]
    fn url_with_userinfo_passes() {
        assert!(validate_url("https://user:pass@example.com/doc.pdf").is_ok());
    }
}
