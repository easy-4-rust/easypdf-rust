//! Security guards that reject malicious inputs before they reach the parser.
//!
//! Inspired by the `GuardDecompressionBomb` and `GuardElementExplosion`
//! patterns in [OfficeCLI](https://github.com/nickvdyck/officecli).
//!
//! These functions are cheap pre-flight checks that inspect metadata
//! (sizes, counts) without inflating any data.

use crate::{PdfError, Result};

use crate::ResourceLimits;

/// Absolute decompressed size (bytes) below which the ratio check is
/// always skipped.
///
/// Streams that decompress to less than this value are considered safe
/// regardless of their compression ratio.  This avoids false positives
/// on very small, benign data (e.g., a 100-byte XML snippet that
/// decompresses to 8 KB).
const ABSOLUTE_SAFE_DECOMPRESSED_SIZE: u64 = 10 * 1024;

/// Reject a stream whose decompressed size or compression ratio exceeds the
/// configured limits.
///
/// This is a lightweight pre-flight check: it compares declared sizes only
/// and never inflates any bytes.
///
/// # Parameters
///
/// * `compressed_size`   -- byte count of the compressed data.
/// * `decompressed_size` -- byte count after decompression (from headers or
///   prior inspection).
/// * `limits`            -- the active [`ResourceLimits`].
///
/// # Errors
///
/// Returns [`PdfError::SecurityViolation`] when either the absolute
/// decompressed size or the compression ratio exceeds the limit.
///
/// # Examples
///
/// ```
/// use easypdf_core::io::guards::guard_decompression_bomb;
/// use easypdf_core::ResourceLimits;
///
/// let limits = ResourceLimits::default();
/// // 10:1 ratio on 100 KB compressed -- within default 100:1 limit.
/// assert!(guard_decompression_bomb(100_000, 1_000_000, &limits).is_ok());
///
/// // 200:1 ratio on 100 KB compressed -- exceeds default 100:1 limit.
/// assert!(guard_decompression_bomb(100_000, 20_000_000, &limits).is_err());
/// ```
pub fn guard_decompression_bomb(
    compressed_size: u64,
    decompressed_size: u64,
    limits: &ResourceLimits,
) -> Result<()> {
    // Absolute size check.
    if decompressed_size > limits.max_decompressed_size() {
        return Err(PdfError::SecurityViolation(format!(
            "decompressed size {} bytes exceeds limit {} bytes",
            decompressed_size,
            limits.max_decompressed_size(),
        )));
    }

    // Tiny streams that decompress to a small absolute size are safe
    // regardless of ratio -- avoids false positives on benign XML.
    if decompressed_size < ABSOLUTE_SAFE_DECOMPRESSED_SIZE {
        return Ok(());
    }

    // Ratio check -- always applied when decompressed output exceeds
    // the safe threshold, regardless of compressed size.  This closes
    // the loophole where a small compressed payload (< 64 KB) could
    // bypass the ratio check even when it inflates to a huge size.
    if let Some(raw_ratio) = decompressed_size.checked_div(compressed_size) {
        let ratio = u32::try_from(raw_ratio).unwrap_or(u32::MAX);
        if ratio > limits.max_compression_ratio() {
            return Err(PdfError::SecurityViolation(format!(
                "compression ratio {ratio}x exceeds limit {}x",
                limits.max_compression_ratio(),
            )));
        }
    }

    Ok(())
}

/// Reject a document whose element count exceeds the configured limit.
///
/// A crafted PDF packed with millions of tiny objects stays well under
/// byte/ratio limits yet materialises into huge in-memory structures.
/// This guard catches such element-explosion attacks.
///
/// # Errors
///
/// Returns [`PdfError::SecurityViolation`] when `element_count` exceeds
/// `limits.max_element_count()`.
///
/// # Examples
///
/// ```
/// use easypdf_core::io::guards::guard_element_explosion;
/// use easypdf_core::ResourceLimits;
///
/// let limits = ResourceLimits::default();
/// assert!(guard_element_explosion(1_000, &limits).is_ok());
/// assert!(guard_element_explosion(10_000_000, &limits).is_err());
/// ```
pub fn guard_element_explosion(element_count: usize, limits: &ResourceLimits) -> Result<()> {
    if element_count > limits.max_element_count() {
        return Err(PdfError::SecurityViolation(format!(
            "element count {} exceeds limit {}",
            element_count,
            limits.max_element_count(),
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- guard_decompression_bomb ---

    #[test]
    fn decompression_within_limits_passes() {
        let limits = ResourceLimits::default();
        // 10:1 ratio, well within 100:1 limit.
        assert!(guard_decompression_bomb(1_000, 10_000, &limits).is_ok());
    }

    #[test]
    fn decompression_exact_limit_passes() {
        let limits = ResourceLimits::default();
        let max = limits.max_decompressed_size();
        assert!(guard_decompression_bomb(max, max, &limits).is_ok());
    }

    #[test]
    fn decompression_size_exceeded_rejected() {
        let limits = ResourceLimits::default();
        let over = limits.max_decompressed_size() + 1;
        let result = guard_decompression_bomb(1_000, over, &limits);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("decompressed size"),
            "unexpected message: {msg}"
        );
    }

    #[test]
    fn decompression_ratio_exceeded_rejected() {
        let limits = ResourceLimits::default();
        // 200:1 ratio on 100 KB compressed -- exceeds default 100:1.
        let result = guard_decompression_bomb(100_000, 20_000_000, &limits);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("compression ratio"),
            "unexpected message: {msg}"
        );
    }

    #[test]
    fn decompression_ratio_applied_for_small_compressed_large_decompressed() {
        // 100 bytes -> 1 MB = 10,000:1 ratio.  Decompressed exceeds the
        // 10 KB safe threshold, so the ratio check now applies and rejects.
        let limits = ResourceLimits::default();
        assert!(guard_decompression_bomb(100, 1_000_000, &limits).is_err());
    }

    #[test]
    fn decompression_ratio_skipped_for_tiny_decompressed() {
        // Both compressed and decompressed are under the 10 KB safe
        // threshold -- ratio check is skipped to avoid false positives.
        let limits = ResourceLimits::default();
        assert!(guard_decompression_bomb(100, 5_000, &limits).is_ok());
    }

    #[test]
    fn decompression_strict_limits_are_tighter() {
        let limits = ResourceLimits::strict();
        // 50:1 ratio is the strict limit.
        assert!(guard_decompression_bomb(100_000, 5_000_000, &limits).is_ok());
        assert!(guard_decompression_bomb(100_000, 6_000_000, &limits).is_err());
    }

    #[test]
    fn decompression_zero_compressed_size_passes() {
        let limits = ResourceLimits::default();
        // Zero compressed size -- ratio check skipped (division guard).
        assert!(guard_decompression_bomb(0, 1_000, &limits).is_ok());
    }

    // --- guard_element_explosion ---

    #[test]
    fn element_within_limits_passes() {
        let limits = ResourceLimits::default();
        assert!(guard_element_explosion(1_000, &limits).is_ok());
    }

    #[test]
    fn element_exact_limit_passes() {
        let limits = ResourceLimits::default();
        assert!(guard_element_explosion(limits.max_element_count(), &limits).is_ok());
    }

    #[test]
    fn element_exceeded_rejected() {
        let limits = ResourceLimits::default();
        let over = limits.max_element_count() + 1;
        let result = guard_element_explosion(over, &limits);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("element count"), "unexpected message: {msg}");
    }

    #[test]
    fn element_strict_limits_are_tighter() {
        let limits = ResourceLimits::strict();
        assert!(guard_element_explosion(999_999, &limits).is_ok());
        assert!(guard_element_explosion(1_000_001, &limits).is_err());
    }

    #[test]
    fn security_violation_error_code() {
        let limits = ResourceLimits::default();
        let err = guard_element_explosion(usize::MAX, &limits).unwrap_err();
        assert_eq!(err.code(), crate::PdfErrorCode::SecurityViolation);
    }
}
