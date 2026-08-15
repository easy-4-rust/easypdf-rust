//! 基于令牌桶算法的限流。
//!
//! 提供线程安全的令牌桶，阻塞直到令牌可用。
//! 用于对云端 OCR API 实施每秒请求限制。
#![cfg_attr(test, allow(clippy::float_cmp))]

use std::time::Instant;

use parking_lot::Mutex;

/// 限流配置。
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// 最大持续每秒请求数。
    pub requests_per_second: f64,
    /// 最大突发大小（可累积的令牌数）。
    pub burst: u32,
}

/// 线程安全的令牌桶限流器。
///
/// 令牌以恒定速率添加，直到达到 `capacity`。每次调用
/// [`acquire`](TokenBucket::acquire) 消耗一个令牌，若无可用令牌则阻塞。
///
/// 使用 `parking_lot::Mutex` 避免中毒问题。
pub struct TokenBucket {
    /// 令牌生成速率（每秒令牌数）。
    rate: f64,
    /// 最大令牌容量。
    capacity: f64,
    /// 当前可用令牌数。
    tokens: Mutex<f64>,
    /// 上次令牌补充时间。
    last: Mutex<Instant>,
}

impl std::fmt::Debug for TokenBucket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenBucket")
            .field("rate", &self.rate)
            .field("capacity", &self.capacity)
            .finish_non_exhaustive()
    }
}

impl TokenBucket {
    /// 根据给定配置创建令牌桶。
    #[must_use]
    pub fn new(config: &RateLimitConfig) -> Self {
        Self {
            rate: config.requests_per_second,
            capacity: f64::from(config.burst),
            tokens: Mutex::new(f64::from(config.burst)),
            last: Mutex::new(Instant::now()),
        }
    }

    /// 阻塞直到令牌可用，然后消耗一个令牌。
    ///
    /// 此方法在循环中休眠，根据经过的时间补充令牌。
    pub fn acquire(&self) {
        loop {
            let wait = {
                let mut tokens = self.tokens.lock();
                let mut last = self.last.lock();

                let now = Instant::now();
                let elapsed = now.duration_since(*last).as_secs_f64();
                *last = now;

                // Refill tokens based on elapsed time.
                *tokens = (*tokens + elapsed * self.rate).min(self.capacity);

                if *tokens >= 1.0 {
                    *tokens -= 1.0;
                    None
                } else {
                    // How long until we have 1 token?
                    Some(Duration::from_secs_f64((1.0 - *tokens) / self.rate))
                }
            };

            match wait {
                Some(duration) => {
                    std::thread::sleep(duration);
                    // Loop back to try acquiring again.
                }
                None => return,
            }
        }
    }
}

use std::time::Duration;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_token_bucket_new() {
        let bucket = TokenBucket::new(&RateLimitConfig {
            requests_per_second: 10.0,
            burst: 5,
        });
        assert_eq!(bucket.rate, 10.0);
        assert_eq!(bucket.capacity, 5.0);
    }

    #[test]
    fn test_token_bucket_acquire_burst() {
        // Burst of 3: first 3 acquires should be instant.
        let bucket = TokenBucket::new(&RateLimitConfig {
            requests_per_second: 100.0, // high rate so refill doesn't interfere
            burst: 3,
        });

        let start = Instant::now();
        bucket.acquire();
        bucket.acquire();
        bucket.acquire();
        let elapsed = start.elapsed();

        // All 3 should complete nearly instantly (< 100ms).
        assert!(
            elapsed < Duration::from_millis(100),
            "burst of 3 took {elapsed:?}"
        );
    }

    #[test]
    fn test_token_bucket_acquire_blocks_after_burst() {
        // Burst of 1, rate of 10/s: second acquire should block ~100ms.
        let bucket = TokenBucket::new(&RateLimitConfig {
            requests_per_second: 10.0,
            burst: 1,
        });

        bucket.acquire(); // uses the burst token
        let start = Instant::now();
        bucket.acquire(); // should block
        let elapsed = start.elapsed();

        // Should have waited approximately 100ms (1/10 second).
        assert!(
            elapsed >= Duration::from_millis(50),
            "expected block, elapsed {elapsed:?}"
        );
        assert!(
            elapsed < Duration::from_millis(500),
            "blocked too long: {elapsed:?}"
        );
    }

    #[test]
    fn test_token_bucket_high_rate_no_block() {
        let bucket = TokenBucket::new(&RateLimitConfig {
            requests_per_second: 1000.0,
            burst: 10,
        });

        let start = Instant::now();
        for _ in 0..10 {
            bucket.acquire();
        }
        let elapsed = start.elapsed();

        // All 10 should complete within the burst window.
        assert!(
            elapsed < Duration::from_millis(200),
            "10 acquires took {elapsed:?}"
        );
    }

    #[test]
    fn test_rate_limit_config_clone() {
        let config = RateLimitConfig {
            requests_per_second: 5.0,
            burst: 3,
        };
        let config2 = config.clone();
        assert_eq!(config2.requests_per_second, 5.0);
        assert_eq!(config2.burst, 3);
    }
}
