//! OAuth 2.0 token management for Baidu Cloud.
//!
//! Baidu Cloud uses OAuth 2.0 client-credentials flow to obtain access tokens.
//! Tokens are cached and reused until they expire. The [`TokenManager`] handles
//! the exchange and caching transparently.
//!
//! # Token Lifetime
//!
//! Baidu access tokens are valid for approximately 30 days (2,592,000 seconds).
//! The [`TokenManager`] refreshes the token when it is within 1 hour of expiry.

use std::time::{Duration, Instant};

use parking_lot::Mutex;

use super::config::{BaiduError, BaiduResult};

/// Default token lifetime assumed by Baidu Cloud: 30 days in seconds.
const DEFAULT_TOKEN_LIFETIME_SECS: u64 = 2_592_000;
/// Refresh threshold: refresh 1 hour before expiry.
const REFRESH_MARGIN_SECS: u64 = 3_600;

/// Cached OAuth token with its expiry time.
#[derive(Debug, Clone)]
struct CachedToken {
    /// The access token string.
    token: String,
    /// When the token was obtained (for expiry tracking).
    obtained_at: Instant,
    /// Token lifetime in seconds.
    lifetime_secs: u64,
}

impl CachedToken {
    /// Check if the token is still valid (not within refresh margin).
    fn is_valid(&self) -> bool {
        let elapsed = self.obtained_at.elapsed();
        let threshold = Duration::from_secs(self.lifetime_secs.saturating_sub(REFRESH_MARGIN_SECS));
        elapsed < threshold
    }
}

/// Thread-safe OAuth token manager for Baidu Cloud.
///
/// Caches the access token and refreshes it automatically. Uses
/// `parking_lot::Mutex` for contention-free locking.
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
    /// OAuth token endpoint URL.
    token_url: String,
    /// API key (client ID).
    api_key: String,
    /// Secret key (client secret).
    secret_key: String,
    /// Cached token.
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
    /// Create a new token manager.
    ///
    /// The token is not fetched until [`get_token`](Self::get_token) is called.
    #[must_use]
    pub fn new(token_url: String, api_key: String, secret_key: String) -> Self {
        Self {
            token_url,
            api_key,
            secret_key,
            cache: Mutex::new(None),
        }
    }

    /// Get a valid access token, refreshing if necessary.
    ///
    /// # Errors
    ///
    /// Returns [`BaiduError::Auth`] if the token exchange fails.
    pub fn get_token(&self) -> BaiduResult<String> {
        // Fast path: check cache under lock.
        {
            let cache = self.cache.lock();
            if let Some(ref cached) = *cache
                && cached.is_valid()
            {
                return Ok(cached.token.clone());
            }
        }

        // Slow path: fetch a new token.
        let token = self.fetch_token()?;
        let cached = CachedToken {
            token: token.clone(),
            obtained_at: Instant::now(),
            lifetime_secs: DEFAULT_TOKEN_LIFETIME_SECS,
        };
        *self.cache.lock() = Some(cached);
        Ok(token)
    }

    /// Exchange API key + secret for an access token via OAuth client credentials.
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

        // Check for error in the response.
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

    /// Invalidate the cached token, forcing a refresh on the next call.
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
