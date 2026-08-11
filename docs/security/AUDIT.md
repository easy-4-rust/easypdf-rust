# Security Audit Report

**Date**: 2026-08-11
**Auditor**: Rust Development Engineer (automated)
**Scope**: easypdf-rust guard validation and API key leakage
**Toolchain**: Rust 1.88, macOS Darwin 25.5.0 arm64

---

## Audit Scope

| Area | Files Audited | Method |
|------|---------------|--------|
| Decompression bomb guard | `easypdf-core/src/io/guards.rs` | Attack vector tests + code review |
| Element explosion guard | `easypdf-core/src/io/guards.rs` | Attack vector tests + code review |
| SSRF URL validation | `easypdf-core/src/io/ssrf_guard.rs` | Attack vector tests + code review |
| API key leakage | `easypdf-ocr/src/*/config.rs`, `easypdf-ocr/src/http/auth.rs` | Static code review (Debug impls) |

---

## A. Decompression Bomb Guard

Guard: `guard_decompression_bomb(compressed_size, decompressed_size, &limits) -> Result<()>`

### Default limits

| Parameter | Default | Strict |
|-----------|---------|--------|
| `max_decompressed_size` | 2 GB | 512 MB |
| `max_compression_ratio` | 100:1 | 50:1 |
| `MIN_COMPRESSED_FOR_RATIO_CHECK` | 64 KB | 64 KB (constant) |

### Test results

| Test | Input | Expected | Result |
|------|-------|----------|--------|
| High ratio 200:1 | 100 KB -> 20 MB | Reject | PASS |
| Extreme ratio 10000:1 | 1 MB -> 10 GB | Reject | PASS |
| Nested compression 1000:1 | 100 KB -> 100 MB | Reject | PASS |
| Strict limits 60:1 | 100 KB -> 6 MB (strict) | Reject | PASS |
| Boundary exact limit | 2 GB -> 2 GB | Pass | PASS |
| Boundary 1 over | 2 GB+1 -> 2 GB+1 | Reject | PASS |
| Small data ratio skip | 100 B -> 5 KB (50:1) | Pass (under 10 KB safe threshold) | PASS |
| Small data ratio applied | 100 B -> 1 MB (10000:1) | Reject (ratio check applies) | PASS |
| Zero compressed | 0 -> 1 MB | Pass (no panic) | PASS |
| Error code check | 100 KB -> 100 MB | SecurityViolation | PASS |
| **Small zip bomb fixed** | **1 KB -> 1 GB (1000000:1)** | **Reject** | **FIXED** |

### FINDING 1: Small compressed payload bypasses ratio check [MEDIUM] -- FIXED

**Location**: `easypdf-core/src/io/guards.rs`

**Description**: The guard skipped the compression ratio check when `compressed_size <= MIN_COMPRESSED_FOR_RATIO_CHECK` (64 KB). This meant a 1 KB compressed payload claiming 1 GB decompressed (1,000,000:1 ratio) passed the guard because the ratio check was skipped entirely for small inputs.

**Fix applied**: Removed the `MIN_COMPRESSED_FOR_RATIO_CHECK` threshold. The guard now uses an absolute safe decompressed size threshold (10 KB) instead:
- If `decompressed_size < 10 KB`: skip ratio check (truly tiny, safe regardless of ratio)
- If `decompressed_size >= 10 KB`: always check the ratio, regardless of `compressed_size`
- Uses `checked_div` to handle zero `compressed_size` without panic

**Regression test**: `audit_a10_small_zip_bomb_ratio_now_blocked` verifies that 1 KB -> 1 GB (1,000,000:1) is now rejected.

---

## B. Element Explosion Guard

Guard: `guard_element_explosion(element_count, &limits) -> Result<()>`

### Default limits

| Parameter | Default | Strict |
|-----------|---------|--------|
| `max_element_count` | 5,000,000 | 1,000,000 |

### Test results

| Test | Input | Expected | Result |
|------|-------|----------|--------|
| 10M elements | 10,000,000 | Reject | PASS |
| 100K elements | 100,000 | Pass | PASS |
| Strict 2M | 2,000,000 (strict) | Reject | PASS |
| Strict tighter than default | Verifies all limits | All stricter | PASS |
| Boundary exact | 5,000,000 | Pass | PASS |
| Boundary 1 over | 5,000,001 | Reject | PASS |
| Error code | usize::MAX | SecurityViolation | PASS |

### Assessment

The element explosion guard works correctly. The boundary conditions are handled properly (inclusive limit). The strict limits are approximately 1/4 of default, as documented.

**No vulnerabilities found.**

---

## C. SSRF URL Validation

Guard: `validate_url(url: &str) -> Result<()>`

### Blocking rules

1. Scheme must be `http` or `https`
2. Host must not be empty
3. Host must not match blocked hostnames: `localhost`, `0.0.0.0`, `127.0.0.1`, `metadata.google.internal`, `169.254.169.254`
4. Host must not parse as private/loopback IPv4: `127.0.0.0/8`, `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `169.254.0.0/16`, `0.0.0.0/8`

### Test results (35 attack URLs, including IPv6)

| Category | URLs tested | All rejected? |
|----------|-------------|---------------|
| Blocked schemes (file/ftp/gopher/javascript/data) | 5 | YES |
| Blocked hostnames (localhost, metadata) | 4 | YES |
| Loopback IPs (127.x.x.x) | 3 | YES |
| Private 10.x.x.x | 2 | YES |
| Private 172.16-31.x.x | 2 | YES |
| Private 192.168.x.x | 2 | YES |
| Link-local 169.254.x.x | 2 | YES |
| Zero network 0.x.x.x | 2 | YES |
| Malformed (no scheme, empty host) | 2 | YES |
| IPv6 loopback (`[::1]`) | 2 | YES |
| IPv6 unspecified (`[::]`) | 1 | YES |
| IPv4-mapped IPv6 (`[::ffff:x.x.x.x]`) | 3 | YES |
| IPv6 ULA (`[fc00::1]`, `[fd00::1]`) | 2 | YES |
| IPv6 link-local (`[fe80::1]`) | 2 | YES |

### Test results (9 legitimate URLs)

| URLs tested | All allowed? |
|-------------|-------------|
| HTTPS/HTTP public hosts, with ports, with query strings, public IPs | YES |

### FINDING 2: IPv6 loopback SSRF bypass [HIGH] -- FIXED

**Location**: `easypdf-core/src/io/ssrf_guard.rs`

**Description**: The SSRF guard only checked IPv4 private ranges via `is_private_ipv4()`. IPv6 addresses like `::1` (loopback) were not checked, allowing attackers to bypass SSRF protection using IPv6 notation.

**Fix applied**: Added comprehensive IPv6 validation using `std::net::IpAddr` parsing:
- `is_blocked_ipv6()` checks: loopback (`::1`), unspecified (`::`), ULA (`fc00::/7`), link-local (`fe80::/10`)
- IPv4-mapped IPv6 addresses (`::ffff:x.x.x.x`) are unwrapped and the embedded IPv4 is checked by `is_private_ipv4_addr()`
- `is_private_ipv4_addr()` uses `std::net::Ipv4Addr` methods plus explicit `0.0.0.0/8` range check

**Blocked URLs** (all now correctly rejected):
- `http://[::1]/` -- IPv6 loopback
- `http://[::]/` -- IPv6 unspecified
- `http://[::ffff:127.0.0.1]/` -- IPv4-mapped loopback
- `http://[::ffff:10.0.0.1]/` -- IPv4-mapped private
- `http://[::ffff:169.254.169.254]/` -- IPv4-mapped metadata
- `http://[fc00::1]/`, `http://[fd00::1]/` -- ULA
- `http://[fe80::1]/` -- link-local

**Regression tests**: `audit_c3` through `audit_c8` verify all IPv6 attack vectors are blocked.

---

## D. API Key Leakage (Static Code Review)

Reviewed all `Debug` implementations on config types containing secrets.

### Test results

| Type | File | `Debug` leaks secrets? | Status |
|------|------|----------------------|--------|
| `HunyuanConfig` | `easypdf-ocr/src/hunyuan/config.rs` | NO -- custom `Debug` redacts `secret_id` and `secret_key` | SAFE |
| `AuthMethod::Bearer` | `easypdf-ocr/src/http/auth.rs` | NO -- shows `***` instead of token | SAFE |
| `AuthMethod::ApiKeyHeader` | `easypdf-ocr/src/http/auth.rs` | NO -- shows `***` instead of key | SAFE |
| `AuthMethod::BearerFromOAuth` | `easypdf-ocr/src/http/auth.rs` | NO -- redacts `secret_key`, shows `api_key` (client ID) | SAFE |
| `AuthMethod::TencentCloud` | `easypdf-ocr/src/http/auth.rs` | NO -- redacts `secret_id` (first4...last4) and `secret_key` (`***`) | SAFE |
| `OcrHttpClient` | `easypdf-ocr/src/http/client.rs` | NO -- delegates to `AuthMethod::Debug` | SAFE |
| `HttpClientConfig` | `easypdf-ocr/src/http/client.rs` | NO -- no secrets in struct | SAFE |

### FINDING 3: GlmConfig leaks API key in Debug output [HIGH] -- FIXED

**Location**: `easypdf-ocr/src/glm/config.rs`

**Description**: `GlmConfig` used `#[derive(Debug)]` which included the `api_key` field in plain text in debug output.

**Fix applied**: Replaced `#[derive(Debug)]` with `#[derive(Clone)]` and added a manual `Debug` impl that redacts `api_key` as `"***redacted***"`, matching the pattern used by `HunyuanConfig`.

**Regression test**: `audit_d1_glm_config_redacts_api_key` verifies the API key does not appear in Debug output.

### FINDING 4: BaiduConfig leaks API key and secret key in Debug output [HIGH] -- FIXED

**Location**: `easypdf-ocr/src/baidu/config.rs`

**Description**: `BaiduConfig` used `#[derive(Debug)]` which included both `api_key` and `secret_key` in plain text.

**Fix applied**: Replaced `#[derive(Debug)]` with `#[derive(Clone)]` and added a manual `Debug` impl that redacts both `api_key` and `secret_key` as `"***redacted***"`, matching the pattern used by `HunyuanConfig`.

**Regression test**: `audit_d2_baidu_config_redacts_both_keys` verifies neither key appears in Debug output.

---

## Summary of Findings

| # | Finding | Severity | Location | Status |
|---|---------|----------|----------|--------|
| 1 | Small compressed payload bypasses ratio check | MEDIUM | `guards.rs` | **FIXED** -- absolute safe threshold + ratio always checked |
| 2 | IPv6 loopback SSRF bypass | HIGH | `ssrf_guard.rs` | **FIXED** -- `std::net::IpAddr` parsing + IPv6 range checks |
| 3 | GlmConfig leaks API key in Debug | HIGH | `glm/config.rs` | **FIXED** -- manual `Debug` with redaction |
| 4 | BaiduConfig leaks API key + secret in Debug | HIGH | `baidu/config.rs` | **FIXED** -- manual `Debug` with redaction |

### What works well

- Element explosion guard is solid with proper boundary handling
- SSRF guard now correctly blocks all IPv4 and IPv6 private/loopback ranges
- Decompression bomb guard now catches small-payload bombs via absolute safe threshold
- All config types with secrets (HunyuanConfig, GlmConfig, BaiduConfig, AuthMethod) have proper Debug redaction
- OcrHttpClient delegates Debug redaction correctly
- All guards return `PdfError::SecurityViolation` with descriptive messages
- 27 security audit regression tests cover all 4 finding areas

---

## Deliverables

1. **Test file**: `easypdf-test/tests/security_audit.rs` -- 27 regression tests covering all 4 audit areas (all passing)
2. **Audit report**: `docs/security/AUDIT.md` -- this file (all 4 findings marked FIXED)
3. **Dependency change**: `easypdf-test/Cargo.toml` -- added `easypdf-ocr` as dev-dependency for Debug redaction tests
4. **Source fixes**: `guards.rs`, `ssrf_guard.rs`, `glm/config.rs`, `baidu/config.rs`
