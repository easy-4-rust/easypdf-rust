//! Retry and backoff strategies for HTTP OCR requests.

use std::time::Duration;

/// Backoff strategy for retry attempts.
///
/// Determines how long to wait between consecutive retries.
#[derive(Debug, Clone)]
pub enum BackoffStrategy {
    /// No retries at all.
    None,
    /// Fixed delay between retries.
    Fixed(Duration),
    /// Exponential backoff: `base_ms * 2^attempt`, capped at `max_ms`.
    Exponential {
        /// Base delay in milliseconds.
        base_ms: u64,
        /// Maximum delay in milliseconds.
        max_ms: u64,
    },
    /// Linear backoff: `step_ms * attempt`, capped at `max_ms`.
    Linear {
        /// Delay increment per attempt in milliseconds.
        step_ms: u64,
        /// Maximum delay in milliseconds.
        max_ms: u64,
    },
}

impl BackoffStrategy {
    /// Calculate the delay before the given retry attempt (0-indexed).
    ///
    /// For `BackoffStrategy::None`, always returns zero.
    #[must_use]
    pub fn delay_for(&self, attempt: u32) -> Duration {
        match self {
            Self::None => Duration::ZERO,
            Self::Fixed(d) => *d,
            Self::Exponential { base_ms, max_ms } => {
                let delay = base_ms.saturating_mul(1u64 << attempt.min(20));
                Duration::from_millis(delay.min(*max_ms))
            }
            Self::Linear { step_ms, max_ms } => {
                let delay = step_ms.saturating_mul(u64::from(attempt));
                Duration::from_millis(delay.min(*max_ms))
            }
        }
    }
}

/// Check if an HTTP status code is retryable.
///
/// Returns `true` for:
/// - 408 Request Timeout
/// - 429 Too Many Requests
/// - 500-599 Server errors
#[must_use]
pub fn is_retryable(status: u16) -> bool {
    matches!(status, 408 | 429 | 500..=599)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_none_delay_is_zero() {
        let strategy = BackoffStrategy::None;
        assert_eq!(strategy.delay_for(0), Duration::ZERO);
        assert_eq!(strategy.delay_for(5), Duration::ZERO);
    }

    #[test]
    fn test_backoff_fixed_delay() {
        let strategy = BackoffStrategy::Fixed(Duration::from_secs(1));
        assert_eq!(strategy.delay_for(0), Duration::from_secs(1));
        assert_eq!(strategy.delay_for(1), Duration::from_secs(1));
        assert_eq!(strategy.delay_for(10), Duration::from_secs(1));
    }

    #[test]
    fn test_backoff_exponential() {
        let strategy = BackoffStrategy::Exponential {
            base_ms: 500,
            max_ms: 8000,
        };
        assert_eq!(strategy.delay_for(0), Duration::from_millis(500));  // 500 * 2^0 = 500
        assert_eq!(strategy.delay_for(1), Duration::from_secs(1)); // 500 * 2^1 = 1000
        assert_eq!(strategy.delay_for(2), Duration::from_secs(2)); // 500 * 2^2 = 2000
        assert_eq!(strategy.delay_for(3), Duration::from_secs(4)); // 500 * 2^3 = 4000
        assert_eq!(strategy.delay_for(4), Duration::from_secs(8)); // 500 * 2^4 = 8000, capped
        assert_eq!(strategy.delay_for(5), Duration::from_secs(8)); // capped at max_ms
    }

    #[test]
    fn test_backoff_exponential_overflow_protection() {
        let strategy = BackoffStrategy::Exponential {
            base_ms: 1000,
            max_ms: 5000,
        };
        // 1000 * 2^30 would overflow, but saturating_mul prevents it.
        let delay = strategy.delay_for(30);
        assert_eq!(delay, Duration::from_secs(5));
    }

    #[test]
    fn test_backoff_linear() {
        let strategy = BackoffStrategy::Linear {
            step_ms: 1000,
            max_ms: 5000,
        };
        assert_eq!(strategy.delay_for(0), Duration::from_millis(0));
        assert_eq!(strategy.delay_for(1), Duration::from_secs(1));
        assert_eq!(strategy.delay_for(2), Duration::from_secs(2));
        assert_eq!(strategy.delay_for(5), Duration::from_secs(5));
        assert_eq!(strategy.delay_for(6), Duration::from_secs(5)); // capped
    }

    #[test]
    fn test_is_retryable_408() {
        assert!(is_retryable(408));
    }

    #[test]
    fn test_is_retryable_429() {
        assert!(is_retryable(429));
    }

    #[test]
    fn test_is_retryable_500() {
        assert!(is_retryable(500));
    }

    #[test]
    fn test_is_retryable_503() {
        assert!(is_retryable(503));
    }

    #[test]
    fn test_is_retryable_599() {
        assert!(is_retryable(599));
    }

    #[test]
    fn test_not_retryable_400() {
        assert!(!is_retryable(400));
    }

    #[test]
    fn test_not_retryable_401() {
        assert!(!is_retryable(401));
    }

    #[test]
    fn test_not_retryable_404() {
        assert!(!is_retryable(404));
    }

    #[test]
    fn test_not_retryable_200() {
        assert!(!is_retryable(200));
    }
}
