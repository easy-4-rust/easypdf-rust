//! Shared HTTP client infrastructure for OCR engine integrations.
//!
//! This module provides a common foundation for cloud-based OCR engines used
//! in the easypdf pipeline. It handles HTTP transport, authentication, retry,
//! rate limiting, and image encoding, allowing each engine to focus only on
//! its specific request/response format.

pub mod auth;
pub mod client;
pub mod error;
pub mod image;
pub mod rate_limit;
pub mod request;
pub mod response;
pub mod retry;

use easypdf_core::CapabilityLevel;
use easypdf_markdown::ocr::{OcrEngine, OcrImage, OcrResult};

use self::client::{HttpClientConfig, OcrHttpClient};
use self::error::Result;
use self::request::{OcrRequest, RequestConfig};
use self::response::OcrResponseParser;

// Re-exports for convenience.
pub use auth::AuthMethod as Auth;
pub use client::HttpClientConfig as Config;
pub use error::OcrHttpError;
pub use image::{EncodedImage, ImageEncoding};
pub use rate_limit::RateLimitConfig;
pub use retry::BackoffStrategy;

/// Generic HTTP-based OCR engine.
///
/// Combines an [`OcrRequest`] (builds the engine-specific request body)
/// with an [`OcrResponseParser`] (parses the engine-specific response)
/// and an [`OcrHttpClient`] (handles transport, auth, retry, rate limiting).
///
/// This struct implements [`OcrEngine`] from `easypdf-markdown-ocr`, so it
/// can be used directly in the OCR processor pipeline.
pub struct HttpOcrEngine<R: OcrRequest, P: OcrResponseParser> {
    request: R,
    parser: P,
    client: OcrHttpClient,
    request_config: RequestConfig,
}

impl<R: OcrRequest, P: OcrResponseParser> std::fmt::Debug for HttpOcrEngine<R, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpOcrEngine")
            .field("engine_name", &self.request.engine_name())
            .field("endpoint", &self.request.endpoint())
            .field("client", &self.client)
            .finish_non_exhaustive()
    }
}

impl<R: OcrRequest, P: OcrResponseParser> HttpOcrEngine<R, P> {
    /// Create a new HTTP OCR engine with default configuration.
    ///
    /// # Errors
    ///
    /// Returns `OcrHttpError::Transport` if the HTTP client cannot be built.
    pub fn new(request: R, parser: P) -> Result<Self> {
        let client = OcrHttpClient::new(request.endpoint(), request.auth().clone())?;
        Ok(Self {
            request,
            parser,
            client,
            request_config: RequestConfig::default(),
        })
    }

    /// Create a new HTTP OCR engine with custom configuration.
    ///
    /// # Errors
    ///
    /// Returns `OcrHttpError::Transport` if the HTTP client cannot be built.
    pub fn with_config(
        request: R,
        parser: P,
        config: HttpClientConfig,
        request_config: RequestConfig,
    ) -> Result<Self> {
        let client =
            OcrHttpClient::with_config(request.endpoint(), request.auth().clone(), config)?;
        Ok(Self {
            request,
            parser,
            client,
            request_config,
        })
    }

    /// Get a reference to the underlying HTTP client.
    #[must_use]
    pub fn client(&self) -> &OcrHttpClient {
        &self.client
    }
}

impl<R: OcrRequest, P: OcrResponseParser> OcrEngine for HttpOcrEngine<R, P> {
    fn recognize(
        &self,
        image: &OcrImage,
    ) -> std::result::Result<OcrResult, Box<dyn std::error::Error + Send + Sync>> {
        let body = self
            .request
            .build_request_body(image, &self.request_config)?;
        let extra = self.request.extra_headers();
        let extra_ref = if extra.is_empty() { None } else { Some(&extra) };

        let raw: serde_json::Value = self.client.post_json(&body, extra_ref)?;

        // Check for engine-level errors in the response.
        self.parser.parse_response(&raw).map_err(Into::into)
    }

    fn name(&self) -> &'static str {
        self.request.engine_name()
    }

    fn languages(&self) -> &[&str] {
        self.request.languages()
    }

    fn level(&self) -> CapabilityLevel {
        CapabilityLevel::Cloud
    }
}

/// Build an HTTP OCR engine with default configuration.
///
/// This is the simplest way to create an HTTP-based OCR engine.
///
/// # Errors
///
/// Returns `OcrHttpError::Transport` if the HTTP client cannot be built.
pub fn build_http_engine<R, P>(request: R, parser: P) -> Result<HttpOcrEngine<R, P>>
where
    R: OcrRequest,
    P: OcrResponseParser,
{
    HttpOcrEngine::new(request, parser)
}

/// Build an HTTP OCR engine with custom configuration.
///
/// # Errors
///
/// Returns `OcrHttpError::Transport` if the HTTP client cannot be built.
pub fn build_http_engine_with_config<R, P>(
    request: R,
    parser: P,
    config: HttpClientConfig,
) -> Result<HttpOcrEngine<R, P>>
where
    R: OcrRequest,
    P: OcrResponseParser,
{
    HttpOcrEngine::with_config(request, parser, config, RequestConfig::default())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::similar_names, clippy::float_cmp)]
    use super::auth::AuthMethod;
    use super::*;

    /// A mock request builder for testing.
    struct MockRequest {
        endpoint: String,
        auth: AuthMethod,
    }

    impl OcrRequest for MockRequest {
        fn endpoint(&self) -> &str {
            &self.endpoint
        }

        fn auth(&self) -> &AuthMethod {
            &self.auth
        }

        fn build_request_body(
            &self,
            _image: &OcrImage,
            _config: &RequestConfig,
        ) -> Result<serde_json::Value> {
            Ok(serde_json::json!({ "test": true }))
        }

        fn engine_name(&self) -> &'static str {
            "mock-http"
        }

        fn languages(&self) -> &[&str] {
            &["en"]
        }
    }

    /// A mock response parser for testing.
    struct MockParser;

    impl OcrResponseParser for MockParser {
        fn parse_response(&self, _raw: &serde_json::Value) -> Result<OcrResult> {
            Ok(OcrResult {
                text: "mock result".to_owned(),
                confidence: Some(0.99),
                word_boxes: vec![],
            })
        }
    }

    #[test]
    fn test_build_http_engine() {
        let request = MockRequest {
            endpoint: "https://example.com/ocr".to_owned(),
            auth: AuthMethod::None,
        };
        let engine = build_http_engine(request, MockParser).unwrap();
        assert_eq!(engine.name(), "mock-http");
        assert_eq!(engine.languages(), &["en"]);
        assert_eq!(engine.level(), CapabilityLevel::Cloud);
    }

    #[test]
    fn test_build_http_engine_with_config() {
        let request = MockRequest {
            endpoint: "https://example.com/ocr".to_owned(),
            auth: AuthMethod::None,
        };
        let config = HttpClientConfig {
            max_retries: 1,
            ..HttpClientConfig::default()
        };
        let engine = build_http_engine_with_config(request, MockParser, config).unwrap();
        assert_eq!(engine.name(), "mock-http");
    }

    #[test]
    fn test_http_ocr_engine_debug() {
        let request = MockRequest {
            endpoint: "https://example.com/ocr".to_owned(),
            auth: AuthMethod::Bearer("secret".to_owned()),
        };
        let engine = build_http_engine(request, MockParser).unwrap();
        let debug = format!("{engine:?}");
        assert!(debug.contains("mock-http"));
        assert!(debug.contains("https://example.com/ocr"));
        assert!(!debug.contains("secret"));
    }

    #[test]
    fn test_re_exports() {
        // Verify re-exports are accessible.
        let _ = Auth::None;
        let _ = Config::default();
        let _ = ImageEncoding::Base64Inline;
        let _ = BackoffStrategy::None;
        let _ = RateLimitConfig {
            requests_per_second: 1.0,
            burst: 1,
        };
    }

    #[test]
    fn test_http_ocr_engine_client_accessor() {
        let request = MockRequest {
            endpoint: "https://example.com/ocr".to_owned(),
            auth: AuthMethod::None,
        };
        let engine = build_http_engine(request, MockParser).unwrap();
        let client = engine.client();
        assert_eq!(client.endpoint(), "https://example.com/ocr");
    }

    #[test]
    fn test_http_ocr_engine_recognize_with_mock_server() {
        use std::io::{BufRead, BufReader, Read, Write};
        use std::net::TcpListener;
        use std::thread;

        // Start a mock server.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let response_body = r#"{"text":"hello","confidence":0.95,"words_result":[]}"#;

        let handle = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                    .ok();
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
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    response_body.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.write_all(response_body.as_bytes());
                let _ = stream.flush();
            }
        });

        let request = MockRequest {
            endpoint: format!("http://127.0.0.1:{}", addr.port()),
            auth: AuthMethod::None,
        };
        let config = HttpClientConfig {
            timeout: std::time::Duration::from_secs(5),
            max_retries: 0,
            retry_backoff: BackoffStrategy::None,
            ..HttpClientConfig::default()
        };
        let engine =
            HttpOcrEngine::with_config(request, MockParser, config, RequestConfig::default())
                .unwrap();

        let image = OcrImage::new(1, 1, vec![255u8; 4]);
        let result = engine.recognize(&image);
        assert!(result.is_ok());
        let ocr_result = result.unwrap();
        assert_eq!(ocr_result.text, "mock result");

        handle.join().ok();
    }

    #[test]
    fn test_http_ocr_engine_with_extra_headers() {
        use std::io::{BufRead, BufReader, Read, Write};
        use std::net::TcpListener;
        use std::thread;

        struct MockRequestWithHeaders {
            endpoint: String,
            auth: AuthMethod,
        }

        impl OcrRequest for MockRequestWithHeaders {
            fn endpoint(&self) -> &str {
                &self.endpoint
            }
            fn auth(&self) -> &AuthMethod {
                &self.auth
            }
            fn build_request_body(
                &self,
                _image: &OcrImage,
                _config: &RequestConfig,
            ) -> Result<serde_json::Value> {
                Ok(serde_json::json!({ "test": true }))
            }
            fn extra_headers(&self) -> std::collections::HashMap<String, String> {
                let mut h = std::collections::HashMap::new();
                h.insert("X-Custom".to_owned(), "value".to_owned());
                h
            }
            fn engine_name(&self) -> &'static str {
                "mock-with-headers"
            }
            fn languages(&self) -> &[&str] {
                &["en"]
            }
        }

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let response_body = r#"{"text":"ok","confidence":0.9}"#;

        let handle = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                    .ok();
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
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    response_body.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.write_all(response_body.as_bytes());
                let _ = stream.flush();
            }
        });

        let request = MockRequestWithHeaders {
            endpoint: format!("http://127.0.0.1:{}", addr.port()),
            auth: AuthMethod::None,
        };
        let config = HttpClientConfig {
            timeout: std::time::Duration::from_secs(5),
            max_retries: 0,
            retry_backoff: BackoffStrategy::None,
            ..HttpClientConfig::default()
        };
        let engine =
            HttpOcrEngine::with_config(request, MockParser, config, RequestConfig::default())
                .unwrap();

        let image = OcrImage::new(1, 1, vec![255u8; 4]);
        let result = engine.recognize(&image);
        assert!(result.is_ok());

        handle.join().ok();
    }
}
