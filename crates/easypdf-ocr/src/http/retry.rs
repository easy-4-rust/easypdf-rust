//! HTTP OCR 请求的重试和退避策略。

use std::time::Duration;

/// 重试尝试的退避策略。
///
/// 决定连续重试之间的等待时间。
#[derive(Debug, Clone)]
pub enum BackoffStrategy {
    /// 不重试。
    None,
    /// 重试间固定延迟。
    Fixed(Duration),
    /// 指数退避：`base_ms * 2^attempt`，上限为 `max_ms`。
    Exponential {
        /// 基础延迟（毫秒）。
        base_ms: u64,
        /// 最大延迟（毫秒）。
        max_ms: u64,
    },
    /// 线性退避：`step_ms * attempt`，上限为 `max_ms`。
    Linear {
        /// 每次尝试的延迟增量（毫秒）。
        step_ms: u64,
        /// 最大延迟（毫秒）。
        max_ms: u64,
    },
}

impl BackoffStrategy {
    /// 计算给定重试尝试（从 0 开始）之前的延迟。
    ///
    /// 对于 `BackoffStrategy::None`，始终返回零。
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

/// 检查 HTTP 状态码是否可重试。
///
/// 以下状态码返回 `true`：
/// - 408 请求超时
/// - 429 请求过多
/// - 500-599 服务器错误
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
        assert_eq!(strategy.delay_for(0), Duration::from_millis(500)); // 500 * 2^0 = 500
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
