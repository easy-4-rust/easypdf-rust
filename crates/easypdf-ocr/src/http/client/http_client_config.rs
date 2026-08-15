//! HTTP 客户端配置。

use std::time::Duration;

use super::super::rate_limit::RateLimitConfig;
use super::super::retry::BackoffStrategy;

/// HTTP 客户端配置。
///
/// 控制请求超时、重试策略、限流和 User-Agent 等行为。
#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    /// 请求超时时间。默认：60 秒。
    pub timeout: Duration,
    /// 最大重试次数。默认：3。
    pub max_retries: u32,
    /// 重试退避策略。默认：指数退避（基础 500ms，最大 8s）。
    pub retry_backoff: BackoffStrategy,
    /// 可选的限流配置。
    pub rate_limit: Option<RateLimitConfig>,
    /// User-Agent 请求头值。
    pub user_agent: String,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(60),
            max_retries: 3,
            retry_backoff: BackoffStrategy::Exponential {
                base_ms: 500,
                max_ms: 8000,
            },
            rate_limit: None,
            user_agent: format!("easypdf-ocr/{}", env!("CARGO_PKG_VERSION")),
        }
    }
}
