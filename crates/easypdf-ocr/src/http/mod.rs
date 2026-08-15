//! OCR 引擎共享 HTTP 客户端基础设施。
//!
//! 本模块为 easypdf 流水线中使用的云端 OCR 引擎提供通用基础。它处理
//! HTTP 传输、认证、重试、限流和图像编码，使各引擎只需关注其专用的
//! 请求/响应格式。

pub mod auth;
pub mod builder;
pub mod client;
pub mod error;
pub mod http_ocr_engine;
pub mod image;
pub mod rate_limit;
pub mod request;
pub mod response;
pub mod retry;

// 公共路径重导出，保持 `easypdf_ocr::http::HttpOcrEngine` 等路径不变。
pub use auth::AuthMethod as Auth;
pub use builder::{build_http_engine, build_http_engine_with_config};
pub use client::HttpClientConfig as Config;
pub use error::OcrHttpError;
pub use http_ocr_engine::HttpOcrEngine;
pub use image::{EncodedImage, ImageEncoding};
pub use rate_limit::RateLimitConfig;
pub use retry::BackoffStrategy;

#[cfg(test)]
mod tests {
    #![allow(clippy::similar_names, clippy::float_cmp)]
    use super::auth::AuthMethod;
    use super::builder::{build_http_engine, build_http_engine_with_config};
    use super::client::HttpClientConfig;
    use super::http_ocr_engine::HttpOcrEngine;
    use super::image::ImageEncoding;
    use super::rate_limit::RateLimitConfig;
    use super::request::{OcrRequest, RequestConfig};
    use super::response::OcrResponseParser;
    use super::retry::BackoffStrategy;
    use super::{Auth, Config};
    use easypdf_core::CapabilityLevel;
    use easypdf_markdown::ocr::{OcrEngine, OcrImage, OcrResult};

    /// 测试用模拟请求构建器。
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
        ) -> super::error::Result<serde_json::Value> {
            Ok(serde_json::json!({ "test": true }))
        }

        fn engine_name(&self) -> &'static str {
            "mock-http"
        }

        fn languages(&self) -> &[&str] {
            &["en"]
        }
    }

    /// 测试用模拟响应解析器。
    struct MockParser;

    impl OcrResponseParser for MockParser {
        fn parse_response(&self, _raw: &serde_json::Value) -> super::error::Result<OcrResult> {
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
        // 验证重导出是否可访问。
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

        // 启动模拟服务器。
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let response_body = r#"{"text":"hello","confidence":0.95,"words_result":[]}"#;

        let handle = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                    .ok();
                // 读取请求。
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
            ) -> super::error::Result<serde_json::Value> {
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
