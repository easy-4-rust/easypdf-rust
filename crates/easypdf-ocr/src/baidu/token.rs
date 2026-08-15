//! 百度云 OAuth 2.0 令牌管理。
//!
//! 百度云使用 OAuth 2.0 客户端凭证流程获取访问令牌。
//! 令牌被缓存并在过期前复用。[`TokenManager`] 透明地处理交换和缓存。
//!
//! # 令牌生命周期
//!
//! 百度访问令牌有效期约 30 天（2,592,000 秒）。
//! [`TokenManager`] 在令牌距过期 1 小时内自动刷新。

use std::time::{Duration, Instant};

use parking_lot::Mutex;

use super::config::{BaiduError, BaiduResult};

/// 百度云默认令牌生命周期：30 天（秒）。
const DEFAULT_TOKEN_LIFETIME_SECS: u64 = 2_592_000;
/// 刷新阈值：过期前 1 小时刷新。
const REFRESH_MARGIN_SECS: u64 = 3_600;

/// 带过期时间的缓存 OAuth 令牌。
#[derive(Debug, Clone)]
struct CachedToken {
    /// 访问令牌字符串。
    token: String,
    /// 令牌获取时间（用于过期跟踪）。
    obtained_at: Instant,
    /// 令牌生命周期（秒）。
    lifetime_secs: u64,
}

impl CachedToken {
    /// 检查令牌是否仍然有效（不在刷新阈值内）。
    fn is_valid(&self) -> bool {
        let elapsed = self.obtained_at.elapsed();
        let threshold = Duration::from_secs(self.lifetime_secs.saturating_sub(REFRESH_MARGIN_SECS));
        elapsed < threshold
    }
}

/// 百度云线程安全的 OAuth 令牌管理器。
///
/// 缓存访问令牌并自动刷新。使用 `parking_lot::Mutex` 实现无竞争锁定。
///
/// # Examples
///
/// ```ignore
/// use easypdf_ocr::baidu::token::TokenManager;
///
/// let mgr = TokenManager::new(
///     "https://aip.baidubce.com/oauth/2.0/token".to_owned(),
///     "my-api-key".to_owned(),
///     "my-secret-key".to_owned(),
/// );
/// let token = mgr.get_token()?;
/// ```
pub struct TokenManager {
    /// OAuth 令牌端点 URL。
    token_url: String,
    /// API 密钥（客户端 ID）。
    api_key: String,
    /// 密钥（客户端密钥）。
    secret_key: String,
    /// 缓存的令牌。
    cache: Mutex<Option<CachedToken>>,
}

impl std::fmt::Debug for TokenManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenManager")
            .field("token_url", &self.token_url)
            .field("api_key", &self.api_key)
            .field("secret_key", &"***")
            .field("has_cached_token", &self.cache.lock().is_some())
            .finish()
    }
}

impl TokenManager {
    /// 创建令牌管理器。
    ///
    /// 令牌在调用 [`get_token`](Self::get_token) 时才获取。
    #[must_use]
    pub fn new(token_url: String, api_key: String, secret_key: String) -> Self {
        Self {
            token_url,
            api_key,
            secret_key,
            cache: Mutex::new(None),
        }
    }

    /// 获取有效的访问令牌，必要时刷新。
    ///
    /// # Errors
    ///
    /// 若令牌交换失败，返回 [`BaiduError::Auth`]。
    pub fn get_token(&self) -> BaiduResult<String> {
        // 快速路径：在锁下检查缓存。
        {
            let cache = self.cache.lock();
            if let Some(ref cached) = *cache
                && cached.is_valid()
            {
                return Ok(cached.token.clone());
            }
        }

        // 慢速路径：获取新令牌。
        let token = self.fetch_token()?;
        let cached = CachedToken {
            token: token.clone(),
            obtained_at: Instant::now(),
            lifetime_secs: DEFAULT_TOKEN_LIFETIME_SECS,
        };
        *self.cache.lock() = Some(cached);
        Ok(token)
    }

    /// 通过 OAuth 客户端凭证将 API Key + Secret 交换为访问令牌。
    fn fetch_token(&self) -> BaiduResult<String> {
        let client = reqwest::blocking::Client::new();
        let resp = client
            .post(&self.token_url)
            .form(&[
                ("grant_type", "client_credentials"),
                ("client_id", self.api_key.as_str()),
                ("client_secret", self.secret_key.as_str()),
            ])
            .send()
            .map_err(|e| BaiduError::Auth(format!("OAuth request failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(BaiduError::Auth(format!(
                "OAuth returned status {}",
                resp.status()
            )));
        }

        let body: serde_json::Value = resp
            .json()
            .map_err(|e| BaiduError::Auth(format!("OAuth response parse error: {e}")))?;

        // 检查响应中的错误。
        if let Some(err_code) = body.get("error") {
            let desc = body
                .get("error_description")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(BaiduError::Auth(format!("OAuth error: {err_code}: {desc}")));
        }

        body.get("access_token")
            .and_then(|v| v.as_str())
            .map(String::from)
            .ok_or_else(|| BaiduError::Auth("OAuth response missing access_token".to_owned()))
    }

    /// 使缓存的令牌失效，强制下次调用时刷新。
    pub fn invalidate(&self) {
        *self.cache.lock() = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::thread;

    /// A minimal HTTP mock server for testing `TokenManager::fetch_token`.
    struct MockTokenServer {
        addr: std::net::SocketAddr,
        _handle: thread::JoinHandle<()>,
    }

    impl MockTokenServer {
        /// Start a mock server that returns the given status and JSON body.
        fn start(status: u16, json_body: &str) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            let json_body = json_body.to_owned();

            let handle = thread::spawn(move || {
                listener.set_nonblocking(false).ok();
                for _ in 0..10 {
                    if let Ok((mut stream, _)) = listener.accept() {
                        stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
                        // Read request.
                        let mut reader = BufReader::new(&stream);
                        let mut content_length = 0usize;
                        loop {
                            let mut line = String::new();
                            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                                break;
                            }
                            if line.trim().is_empty() {
                                break;
                            }
                            if line.to_lowercase().starts_with("content-length:") {
                                content_length = line
                                    .split(':')
                                    .nth(1)
                                    .and_then(|v| v.trim().parse().ok())
                                    .unwrap_or(0);
                            }
                        }
                        if content_length > 0 {
                            let mut body_buf = vec![0u8; content_length];
                            let _ = reader.read_exact(&mut body_buf);
                        }

                        let response = format!(
                            "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            json_body.len()
                        );
                        let _ = stream.write_all(response.as_bytes());
                        let _ = stream.write_all(json_body.as_bytes());
                        let _ = stream.flush();
                    }
                }
            });

            Self {
                addr,
                _handle: handle,
            }
        }

        fn url(&self) -> String {
            format!("http://127.0.0.1:{}", self.addr.port())
        }
    }

    #[test]
    fn test_cached_token_is_valid_fresh() {
        let cached = CachedToken {
            token: "test".to_owned(),
            obtained_at: Instant::now(),
            lifetime_secs: DEFAULT_TOKEN_LIFETIME_SECS,
        };
        assert!(cached.is_valid());
    }

    #[test]
    fn test_cached_token_is_valid_expired() {
        let cached = CachedToken {
            token: "test".to_owned(),
            obtained_at: Instant::now()
                .checked_sub(Duration::from_secs(DEFAULT_TOKEN_LIFETIME_SECS))
                .unwrap(),
            lifetime_secs: DEFAULT_TOKEN_LIFETIME_SECS,
        };
        assert!(!cached.is_valid());
    }

    #[test]
    fn test_cached_token_near_expiry() {
        // Token with 1 second left (within refresh margin).
        let cached = CachedToken {
            token: "test".to_owned(),
            obtained_at: Instant::now()
                .checked_sub(Duration::from_secs(
                    DEFAULT_TOKEN_LIFETIME_SECS - REFRESH_MARGIN_SECS - 1,
                ))
                .unwrap(),
            lifetime_secs: DEFAULT_TOKEN_LIFETIME_SECS,
        };
        // Should still be valid (just outside the margin).
        assert!(cached.is_valid());

        let cached2 = CachedToken {
            token: "test".to_owned(),
            obtained_at: Instant::now()
                .checked_sub(Duration::from_secs(
                    DEFAULT_TOKEN_LIFETIME_SECS - REFRESH_MARGIN_SECS + 1,
                ))
                .unwrap(),
            lifetime_secs: DEFAULT_TOKEN_LIFETIME_SECS,
        };
        // Should be invalid (within the margin).
        assert!(!cached2.is_valid());
    }

    #[test]
    fn test_token_manager_debug_redacts_secret() {
        let mgr = TokenManager::new(
            "https://example.com/token".to_owned(),
            "my-key".to_owned(),
            "super-secret".to_owned(),
        );
        let debug = format!("{mgr:?}");
        assert!(!debug.contains("super-secret"));
        assert!(debug.contains("my-key"));
        assert!(debug.contains("***"));
    }

    #[test]
    fn test_token_manager_invalidate() {
        let mgr = TokenManager::new(
            "https://example.com/token".to_owned(),
            "key".to_owned(),
            "secret".to_owned(),
        );
        // Simulate a cached token.
        *mgr.cache.lock() = Some(CachedToken {
            token: "old-token".to_owned(),
            obtained_at: Instant::now(),
            lifetime_secs: DEFAULT_TOKEN_LIFETIME_SECS,
        });
        assert!(mgr.cache.lock().is_some());

        mgr.invalidate();
        assert!(mgr.cache.lock().is_none());
    }

    // --- Integration tests with mock HTTP server ---

    #[test]
    fn test_get_token_fetches_on_first_call() {
        let server = MockTokenServer::start(
            200,
            r#"{"access_token":"fresh-token-abc","expires_in":2592000}"#,
        );
        let mgr = TokenManager::new(
            server.url(),
            "test-key".to_owned(),
            "test-secret".to_owned(),
        );

        let token = mgr.get_token().unwrap();
        assert_eq!(token, "fresh-token-abc");
    }

    #[test]
    fn test_get_token_returns_cached_on_second_call() {
        let server = MockTokenServer::start(
            200,
            r#"{"access_token":"cached-token","expires_in":2592000}"#,
        );
        let mgr = TokenManager::new(
            server.url(),
            "test-key".to_owned(),
            "test-secret".to_owned(),
        );

        // First call fetches from server.
        let token1 = mgr.get_token().unwrap();
        assert_eq!(token1, "cached-token");

        // Second call should return cached (no new HTTP request).
        let token2 = mgr.get_token().unwrap();
        assert_eq!(token2, "cached-token");
    }

    #[test]
    fn test_get_token_refreshes_after_invalidate() {
        let server = MockTokenServer::start(
            200,
            r#"{"access_token":"refreshed-token","expires_in":2592000}"#,
        );
        let mgr = TokenManager::new(
            server.url(),
            "test-key".to_owned(),
            "test-secret".to_owned(),
        );

        // First fetch.
        let token1 = mgr.get_token().unwrap();
        assert_eq!(token1, "refreshed-token");

        // Invalidate forces re-fetch.
        mgr.invalidate();
        let token2 = mgr.get_token().unwrap();
        assert_eq!(token2, "refreshed-token");
    }

    #[test]
    fn test_fetch_token_http_error() {
        let server = MockTokenServer::start(500, "Internal Server Error");
        let mgr = TokenManager::new(
            server.url(),
            "test-key".to_owned(),
            "test-secret".to_owned(),
        );

        let result = mgr.get_token();
        assert!(result.is_err());
        match result.unwrap_err() {
            BaiduError::Auth(msg) => assert!(msg.contains("OAuth returned status")),
            other => panic!("expected Auth error, got: {other:?}"),
        }
    }

    #[test]
    fn test_fetch_token_error_in_response() {
        let server = MockTokenServer::start(
            200,
            r#"{"error":"invalid_client","error_description":"The client identifier is invalid"}"#,
        );
        let mgr = TokenManager::new(server.url(), "bad-key".to_owned(), "bad-secret".to_owned());

        let result = mgr.get_token();
        assert!(result.is_err());
        match result.unwrap_err() {
            BaiduError::Auth(msg) => {
                assert!(msg.contains("OAuth error"));
                assert!(msg.contains("invalid_client"));
            }
            other => panic!("expected Auth error, got: {other:?}"),
        }
    }

    #[test]
    fn test_fetch_token_missing_access_token() {
        let server = MockTokenServer::start(200, r#"{"expires_in":2592000}"#);
        let mgr = TokenManager::new(
            server.url(),
            "test-key".to_owned(),
            "test-secret".to_owned(),
        );

        let result = mgr.get_token();
        assert!(result.is_err());
        match result.unwrap_err() {
            BaiduError::Auth(msg) => assert!(msg.contains("missing access_token")),
            other => panic!("expected Auth error, got: {other:?}"),
        }
    }

    #[test]
    fn test_fetch_token_invalid_json() {
        let server = MockTokenServer::start(200, "not json");
        let mgr = TokenManager::new(
            server.url(),
            "test-key".to_owned(),
            "test-secret".to_owned(),
        );

        let result = mgr.get_token();
        assert!(result.is_err());
        match result.unwrap_err() {
            BaiduError::Auth(msg) => assert!(msg.contains("parse error")),
            other => panic!("expected Auth error, got: {other:?}"),
        }
    }

    #[test]
    fn test_fetch_token_network_error() {
        // Use a port that's not listening.
        let mgr = TokenManager::new(
            "http://127.0.0.1:1".to_owned(),
            "test-key".to_owned(),
            "test-secret".to_owned(),
        );

        let result = mgr.get_token();
        assert!(result.is_err());
        match result.unwrap_err() {
            BaiduError::Auth(msg) => assert!(msg.contains("OAuth request failed")),
            other => panic!("expected Auth error, got: {other:?}"),
        }
    }

    #[test]
    fn test_get_token_concurrent_access() {
        let server = MockTokenServer::start(
            200,
            r#"{"access_token":"concurrent-token","expires_in":2592000}"#,
        );
        let mgr = std::sync::Arc::new(TokenManager::new(
            server.url(),
            "test-key".to_owned(),
            "test-secret".to_owned(),
        ));

        let mut handles = vec![];
        for _ in 0..4 {
            let mgr_clone = std::sync::Arc::clone(&mgr);
            handles.push(thread::spawn(move || {
                let token = mgr_clone.get_token().unwrap();
                assert_eq!(token, "concurrent-token");
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
    }
}
