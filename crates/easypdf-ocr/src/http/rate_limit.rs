//! Rate limiting via token bucket algorithm.
//!
//! Provides a thread-safe token bucket that blocks until a token is available.
//! Used to enforce per-second request limits for cloud OCR APIs.
#![cfg_attr(test, allow(clippy::float_cmp))]

use std::time::Instant;

use parking_lot::Mutex;

/// Rate limit configuration.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum sustained requests per second.
    pub requests_per_second: f64,
    /// Maximum burst size (number of tokens that can accumulate).
    pub burst: u32,
}

/// Thread-safe token bucket rate limiter.
///
/// Tokens are added at a constant rate up to `capacity`. Each call to
/// [`acquire`](TokenBucket::acquire) consumes one token, blocking if none
/// are available.
///
/// Uses `parking_lot::Mutex` to avoid poisoning issues.
pub struct TokenBucket {
    /// Token generation rate (tokens per second).
    rate: f64,
    /// Maximum token capacity.
    capacity: f64,
    /// Current number of available tokens.
    tokens: Mutex<f64>,
    /// Last time tokens were refilled.
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
    /// Create a new token bucket from the given configuration.
    #[must_use]
    pub fn new(config: &RateLimitConfig) -> Self {
        Self {
            rate: config.requests_per_second,
            capacity: f64::from(config.burst),
            tokens: Mutex::new(f64::from(config.burst)),
            last: Mutex::new(Instant::now()),
        }
    }

    /// Block until a token is available, then consume it.
    ///
    /// This method sleeps in a loop, refilling tokens based on elapsed time.
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
