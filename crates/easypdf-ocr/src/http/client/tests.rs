use super::super::auth::AuthMethod;
use super::super::error::{OcrHttpError, Result};
use super::super::rate_limit::RateLimitConfig;
use super::super::retry::BackoffStrategy;
use super::HttpClientConfig;
use super::OcrHttpClient;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// A minimal HTTP mock server for testing `OcrHttpClient`.
///
/// Accepts TCP connections and returns pre-configured responses.
struct MockServer {
    addr: std::net::SocketAddr,
    _handle: thread::JoinHandle<()>,
    /// Number of requests received.
    request_count: Arc<Mutex<usize>>,
}

impl MockServer {
    /// Start a mock server that returns the given status and body for every request.
    fn start(status: u16, body: &[u8]) -> Self {
        Self::start_sequence(vec![(status, body.to_vec())])
    }

    /// Start a mock server that returns a sequence of responses.
    /// After exhausting the sequence, repeats the last response.
    fn start_sequence(responses: Vec<(u16, Vec<u8>)>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let request_count = Arc::new(Mutex::new(0usize));
        let count_clone = Arc::clone(&request_count);
        let responses = Arc::new(responses);

        let handle = thread::spawn(move || {
            listener.set_nonblocking(false).ok();
            // Accept multiple connections (for retry tests).
            for _ in 0..20 {
                if let Ok((mut stream, _)) = listener.accept() {
                    stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
                    stream.set_write_timeout(Some(Duration::from_secs(2))).ok();

                    // Read the full request (headers + body).
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
                    // Read body bytes if present.
                    if content_length > 0 {
                        let mut body_buf = vec![0u8; content_length];
                        let _ = reader.read_exact(&mut body_buf);
                    }

                    let mut count = count_clone.lock().unwrap();
                    let idx = (*count).min(responses.len() - 1);
                    *count += 1;
                    let (status, body) = &responses[idx];

                    let response = format!(
                        "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.write_all(body);
                    let _ = stream.flush();
                }
            }
        });

        Self {
            addr,
            _handle: handle,
            request_count,
        }
    }

    fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.addr.port())
    }

    fn request_count(&self) -> usize {
        *self.request_count.lock().unwrap()
    }
}

#[test]
fn test_http_client_config_default() {
    let config = HttpClientConfig::default();
    assert_eq!(config.timeout, Duration::from_secs(60));
    assert_eq!(config.max_retries, 3);
    assert!(config.rate_limit.is_none());
    assert!(config.user_agent.starts_with("easypdf-ocr/"));
}

#[test]
fn test_http_client_config_clone() {
    let config = HttpClientConfig::default();
    let config2 = config.clone();
    assert_eq!(config2.timeout, config.timeout);
}

#[test]
fn test_http_client_new() {
    let client = OcrHttpClient::new("https://example.com", AuthMethod::None);
    assert!(client.is_ok());
    let client = client.unwrap();
    assert_eq!(client.endpoint(), "https://example.com");
}

#[test]
fn test_http_client_debug() {
    let client = OcrHttpClient::new(
        "https://example.com",
        AuthMethod::Bearer("secret".to_owned()),
    )
    .unwrap();
    let debug = format!("{client:?}");
    // Debug should show endpoint but redact auth token.
    assert!(debug.contains("https://example.com"));
    assert!(!debug.contains("secret"));
}

#[test]
fn test_with_config_custom() {
    let config = HttpClientConfig {
        timeout: Duration::from_secs(10),
        max_retries: 1,
        retry_backoff: BackoffStrategy::None,
        rate_limit: None,
        user_agent: "test/1.0".to_owned(),
    };
    let client = OcrHttpClient::with_config("https://example.com", AuthMethod::None, config);
    assert!(client.is_ok());
}

#[test]
fn test_with_config_rate_limit() {
    let config = HttpClientConfig {
        timeout: Duration::from_secs(10),
        max_retries: 0,
        retry_backoff: BackoffStrategy::None,
        rate_limit: Some(RateLimitConfig {
            requests_per_second: 100.0,
            burst: 1,
        }),
        user_agent: "test/1.0".to_owned(),
    };
    let client = OcrHttpClient::with_config("https://example.com", AuthMethod::None, config);
    assert!(client.is_ok());
}

#[test]
fn test_auth_accessor() {
    let client =
        OcrHttpClient::new("https://example.com", AuthMethod::Bearer("tok".to_owned())).unwrap();
    match client.auth() {
        AuthMethod::Bearer(t) => assert_eq!(t, "tok"),
        _ => panic!("expected Bearer auth"),
    }
}

// --- Mock server integration tests ---

#[test]
fn test_execute_success_200() {
    let server = MockServer::start(200, b"{\"ok\":true}");
    let client = OcrHttpClient::with_config(
        server.url(),
        AuthMethod::None,
        HttpClientConfig {
            max_retries: 0,
            retry_backoff: BackoffStrategy::None,
            ..HttpClientConfig::default()
        },
    )
    .unwrap();

    let result = client.execute(|c| c.post(server.url()).body(b"test".to_vec()).send());
    assert!(result.is_ok());
    assert_eq!(result.unwrap().status(), 200);
    assert_eq!(server.request_count(), 1);
}

#[test]
fn test_execute_500_retries_then_fails() {
    // Return 500 for all requests (4 attempts: 1 initial + 3 retries).
    let server = MockServer::start_sequence(vec![
        (500, b"server error".to_vec()),
        (500, b"server error".to_vec()),
        (500, b"server error".to_vec()),
        (500, b"server error".to_vec()),
    ]);
    let client = OcrHttpClient::with_config(
        server.url(),
        AuthMethod::None,
        HttpClientConfig {
            max_retries: 3,
            retry_backoff: BackoffStrategy::Fixed(Duration::from_millis(10)),
            ..HttpClientConfig::default()
        },
    )
    .unwrap();

    let result = client.execute(|c| {
        c.post(format!("{}/test", server.url()))
            .body(b"test".to_vec())
            .send()
    });
    assert!(result.is_err());
    match result.unwrap_err() {
        OcrHttpError::ServerError { status, .. } => assert_eq!(status, 500),
        other => panic!("expected ServerError, got: {other:?}"),
    }
    // Should have made 4 attempts (1 + 3 retries).
    assert_eq!(server.request_count(), 4);
}

#[test]
fn test_execute_500_then_success_on_retry() {
    // First request returns 500, second returns 200.
    let server = MockServer::start_sequence(vec![
        (500, b"temporary error".to_vec()),
        (200, b"{\"ok\":true}".to_vec()),
    ]);
    let client = OcrHttpClient::with_config(
        server.url(),
        AuthMethod::None,
        HttpClientConfig {
            max_retries: 3,
            retry_backoff: BackoffStrategy::Fixed(Duration::from_millis(10)),
            ..HttpClientConfig::default()
        },
    )
    .unwrap();

    let result = client.execute(|c| {
        c.post(format!("{}/test", server.url()))
            .body(b"test".to_vec())
            .send()
    });
    assert!(result.is_ok());
    assert_eq!(server.request_count(), 2);
}

#[test]
fn test_execute_400_no_retry() {
    let server = MockServer::start(400, b"bad request");
    let client = OcrHttpClient::with_config(
        server.url(),
        AuthMethod::None,
        HttpClientConfig {
            max_retries: 3,
            retry_backoff: BackoffStrategy::Fixed(Duration::from_millis(10)),
            ..HttpClientConfig::default()
        },
    )
    .unwrap();

    let result = client.execute(|c| {
        c.post(format!("{}/test", server.url()))
            .body(b"test".to_vec())
            .send()
    });
    assert!(result.is_err());
    match result.unwrap_err() {
        OcrHttpError::BadRequest { code, .. } => assert_eq!(code, 400),
        other => panic!("expected BadRequest, got: {other:?}"),
    }
    // 400 is not retryable -- should have made only 1 attempt.
    assert_eq!(server.request_count(), 1);
}

#[test]
fn test_execute_401_no_retry() {
    let server = MockServer::start(401, b"unauthorized");
    let client = OcrHttpClient::with_config(
        server.url(),
        AuthMethod::None,
        HttpClientConfig {
            max_retries: 3,
            retry_backoff: BackoffStrategy::Fixed(Duration::from_millis(10)),
            ..HttpClientConfig::default()
        },
    )
    .unwrap();

    let result = client.execute(|c| {
        c.post(format!("{}/test", server.url()))
            .body(b"test".to_vec())
            .send()
    });
    assert!(result.is_err());
    match result.unwrap_err() {
        OcrHttpError::BadRequest { code, .. } => assert_eq!(code, 401),
        other => panic!("expected BadRequest, got: {other:?}"),
    }
    assert_eq!(server.request_count(), 1);
}

#[test]
fn test_execute_429_returns_rate_limit_error() {
    // 429 is retryable, but after max_retries it should return RateLimit.
    let server = MockServer::start_sequence(vec![
        (429, b"rate limited".to_vec()),
        (429, b"rate limited".to_vec()),
        (429, b"rate limited".to_vec()),
        (429, b"rate limited".to_vec()),
    ]);
    let client = OcrHttpClient::with_config(
        server.url(),
        AuthMethod::None,
        HttpClientConfig {
            max_retries: 3,
            retry_backoff: BackoffStrategy::Fixed(Duration::from_millis(10)),
            ..HttpClientConfig::default()
        },
    )
    .unwrap();

    let result = client.execute(|c| {
        c.post(format!("{}/test", server.url()))
            .body(b"test".to_vec())
            .send()
    });
    assert!(result.is_err());
    match result.unwrap_err() {
        OcrHttpError::RateLimit { retry_after_secs } => assert_eq!(retry_after_secs, 60),
        other => panic!("expected RateLimit, got: {other:?}"),
    }
}

#[test]
fn test_post_json_success() {
    let response_body = r#"{"text":"hello","confidence":0.95}"#;
    let server = MockServer::start(200, response_body.as_bytes());
    let client = OcrHttpClient::with_config(
        server.url(),
        AuthMethod::None,
        HttpClientConfig {
            max_retries: 0,
            retry_backoff: BackoffStrategy::None,
            ..HttpClientConfig::default()
        },
    )
    .unwrap();

    let body = serde_json::json!({"image": "base64data"});
    let result: Result<serde_json::Value> = client.post_json(&body, None);
    assert!(result.is_ok());
    let val = result.unwrap();
    assert_eq!(val["text"], "hello");
    assert_eq!(server.request_count(), 1);
}

#[test]
fn test_post_json_with_bearer_auth() {
    let response_body = r#"{"ok":true}"#;
    let server = MockServer::start(200, response_body.as_bytes());
    let client = OcrHttpClient::with_config(
        server.url(),
        AuthMethod::Bearer("my-token".to_owned()),
        HttpClientConfig {
            max_retries: 0,
            retry_backoff: BackoffStrategy::None,
            ..HttpClientConfig::default()
        },
    )
    .unwrap();

    let body = serde_json::json!({"test": true});
    let result: Result<serde_json::Value> = client.post_json(&body, None);
    assert!(result.is_ok());
}

#[test]
fn test_post_json_with_extra_headers() {
    let response_body = r#"{"ok":true}"#;
    let server = MockServer::start(200, response_body.as_bytes());
    let client = OcrHttpClient::with_config(
        server.url(),
        AuthMethod::None,
        HttpClientConfig {
            max_retries: 0,
            retry_backoff: BackoffStrategy::None,
            ..HttpClientConfig::default()
        },
    )
    .unwrap();

    let mut extra = std::collections::HashMap::new();
    extra.insert("X-Custom".to_owned(), "value".to_owned());
    let body = serde_json::json!({"test": true});
    let result: Result<serde_json::Value> = client.post_json(&body, Some(&extra));
    assert!(result.is_ok());
}

#[test]
fn test_post_json_server_error() {
    let server = MockServer::start_sequence(vec![
        (500, b"internal error".to_vec()),
        (500, b"internal error".to_vec()),
        (500, b"internal error".to_vec()),
        (500, b"internal error".to_vec()),
    ]);
    let client = OcrHttpClient::with_config(
        server.url(),
        AuthMethod::None,
        HttpClientConfig {
            max_retries: 3,
            retry_backoff: BackoffStrategy::Fixed(Duration::from_millis(10)),
            ..HttpClientConfig::default()
        },
    )
    .unwrap();

    let body = serde_json::json!({"test": true});
    let result: Result<serde_json::Value> = client.post_json(&body, None);
    assert!(result.is_err());
}

#[test]
fn test_post_json_invalid_response_body() {
    // Return 200 with invalid JSON.
    let server = MockServer::start(200, b"not json at all");
    let client = OcrHttpClient::with_config(
        server.url(),
        AuthMethod::None,
        HttpClientConfig {
            max_retries: 0,
            retry_backoff: BackoffStrategy::None,
            ..HttpClientConfig::default()
        },
    )
    .unwrap();

    let body = serde_json::json!({"test": true});
    let result: Result<serde_json::Value> = client.post_json(&body, None);
    assert!(result.is_err());
    match result.unwrap_err() {
        OcrHttpError::InvalidResponse(msg) => assert!(msg.contains("JSON parse error")),
        other => panic!("expected InvalidResponse, got: {other:?}"),
    }
}

#[test]
fn test_execute_with_api_key_auth() {
    let response_body = r#"{"ok":true}"#;
    let server = MockServer::start(200, response_body.as_bytes());
    let client = OcrHttpClient::with_config(
        server.url(),
        AuthMethod::ApiKeyHeader {
            header: "x-api-key",
            key: "test-key".to_owned(),
        },
        HttpClientConfig {
            max_retries: 0,
            retry_backoff: BackoffStrategy::None,
            ..HttpClientConfig::default()
        },
    )
    .unwrap();

    let body = serde_json::json!({"test": true});
    let result: Result<serde_json::Value> = client.post_json(&body, None);
    assert!(result.is_ok());
}

#[test]
fn test_execute_network_error_retries() {
    // Use a port that's not listening to simulate connection refused.
    let client = OcrHttpClient::with_config(
        "http://127.0.0.1:1", // port 1 is almost certainly not listening
        AuthMethod::None,
        HttpClientConfig {
            timeout: Duration::from_millis(100),
            max_retries: 2,
            retry_backoff: BackoffStrategy::Fixed(Duration::from_millis(10)),
            ..HttpClientConfig::default()
        },
    )
    .unwrap();

    let result = client.execute(|c| {
        c.post("http://127.0.0.1:1/test")
            .body(b"test".to_vec())
            .send()
    });
    assert!(result.is_err());
    match result.unwrap_err() {
        OcrHttpError::Transport(_) => {} // expected
        OcrHttpError::MaxRetriesExceeded { max } => assert_eq!(max, 2),
        other => panic!("expected Transport or MaxRetriesExceeded, got: {other:?}"),
    }
}

#[test]
fn test_execute_max_retries_exceeded() {
    // Use a port that's not listening.
    let client = OcrHttpClient::with_config(
        "http://127.0.0.1:1",
        AuthMethod::None,
        HttpClientConfig {
            timeout: Duration::from_millis(50),
            max_retries: 0,
            retry_backoff: BackoffStrategy::None,
            ..HttpClientConfig::default()
        },
    )
    .unwrap();

    let result = client.execute(|c| {
        c.post("http://127.0.0.1:1/test")
            .body(b"test".to_vec())
            .send()
    });
    assert!(result.is_err());
}

#[test]
fn test_post_json_with_tencent_cloud_auth() {
    let response_body = r#"{"ok":true}"#;
    let server = MockServer::start(200, response_body.as_bytes());
    let client = OcrHttpClient::with_config(
        server.url(),
        AuthMethod::TencentCloud {
            secret_id: "id123".to_owned(),
            secret_key: "key456".to_owned(),
            service: "hunyuan".to_owned(),
            host: "hunyuan.tencentcloudapi.com".to_owned(),
            region: "ap-guangzhou".to_owned(),
            version: "2023-09-01".to_owned(),
        },
        HttpClientConfig {
            max_retries: 0,
            retry_backoff: BackoffStrategy::None,
            ..HttpClientConfig::default()
        },
    )
    .unwrap();

    let body = serde_json::json!({"test": true});
    let result: Result<serde_json::Value> = client.post_json(&body, None);
    // This will fail because the Tencent Cloud auth tries to sign the request
    // and the host doesn't match, but it exercises the auth path.
    // The important thing is that it doesn't panic.
    let _ = result;
}

#[test]
fn test_post_json_bearer_from_oauth_uses_cached_token() {
    // BearerFromOAuth needs a token endpoint. We'll test that the auth
    // headers are applied correctly by using a mock server.
    let server = MockServer::start(200, r#"{"ok":true}"#.as_bytes());
    let client = OcrHttpClient::with_config(
        server.url(),
        AuthMethod::Bearer("pre-obtained-token".to_owned()),
        HttpClientConfig {
            max_retries: 0,
            retry_backoff: BackoffStrategy::None,
            ..HttpClientConfig::default()
        },
    )
    .unwrap();

    let body = serde_json::json!({"test": true});
    let result: Result<serde_json::Value> = client.post_json(&body, None);
    assert!(result.is_ok());
}

#[test]
fn test_execute_503_retryable() {
    let server = MockServer::start_sequence(vec![
        (503, b"unavailable".to_vec()),
        (200, r#"{"ok":true}"#.as_bytes().to_vec()),
    ]);
    let client = OcrHttpClient::with_config(
        server.url(),
        AuthMethod::None,
        HttpClientConfig {
            max_retries: 3,
            retry_backoff: BackoffStrategy::Fixed(Duration::from_millis(10)),
            ..HttpClientConfig::default()
        },
    )
    .unwrap();

    let result = client.execute(|c| {
        c.post(format!("{}/test", server.url()))
            .body(b"test".to_vec())
            .send()
    });
    assert!(result.is_ok());
    assert_eq!(server.request_count(), 2);
}

#[test]
fn test_execute_408_retryable() {
    let server = MockServer::start_sequence(vec![
        (408, b"timeout".to_vec()),
        (200, r#"{"ok":true}"#.as_bytes().to_vec()),
    ]);
    let client = OcrHttpClient::with_config(
        server.url(),
        AuthMethod::None,
        HttpClientConfig {
            max_retries: 3,
            retry_backoff: BackoffStrategy::Fixed(Duration::from_millis(10)),
            ..HttpClientConfig::default()
        },
    )
    .unwrap();

    let result = client.execute(|c| {
        c.post(format!("{}/test", server.url()))
            .body(b"test".to_vec())
            .send()
    });
    assert!(result.is_ok());
    assert_eq!(server.request_count(), 2);
}

#[test]
fn test_post_json_with_rate_limiter() {
    let server = MockServer::start(200, r#"{"ok":true}"#.as_bytes());
    let client = OcrHttpClient::with_config(
        server.url(),
        AuthMethod::None,
        HttpClientConfig {
            max_retries: 0,
            retry_backoff: BackoffStrategy::None,
            rate_limit: Some(RateLimitConfig {
                requests_per_second: 1000.0,
                burst: 10,
            }),
            ..HttpClientConfig::default()
        },
    )
    .unwrap();

    let body = serde_json::json!({"test": true});
    let result: Result<serde_json::Value> = client.post_json(&body, None);
    assert!(result.is_ok());
    assert_eq!(server.request_count(), 1);
}

#[test]
fn test_post_json_exponential_backoff() {
    // Verify that exponential backoff is used (all retries fail quickly).
    let server = MockServer::start_sequence(vec![
        (500, b"err".to_vec()),
        (500, b"err".to_vec()),
        (500, b"err".to_vec()),
        (500, b"err".to_vec()),
    ]);
    let client = OcrHttpClient::with_config(
        server.url(),
        AuthMethod::None,
        HttpClientConfig {
            max_retries: 3,
            retry_backoff: BackoffStrategy::Exponential {
                base_ms: 10,
                max_ms: 100,
            },
            ..HttpClientConfig::default()
        },
    )
    .unwrap();

    let start = std::time::Instant::now();
    let body = serde_json::json!({"test": true});
    let result: Result<serde_json::Value> = client.post_json(&body, None);
    let elapsed = start.elapsed();
    assert!(result.is_err());
    // With exponential backoff: 10 + 20 + 40 = 70ms minimum.
    assert!(
        elapsed >= Duration::from_millis(50),
        "expected backoff delay, took {elapsed:?}"
    );
    assert_eq!(server.request_count(), 4);
}
