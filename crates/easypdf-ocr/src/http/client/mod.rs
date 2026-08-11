//! Synchronous HTTP client for OCR API calls.
//!
//! Provides [`OcrHttpClient`], a blocking HTTP client with built-in retry,
//! rate limiting, and authentication. Uses `reqwest::blocking::Client`
//! to avoid requiring an async runtime.

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::auth::{apply_auth, AuthMethod};
use super::error::{OcrHttpError, Result};
use super::rate_limit::{RateLimitConfig, TokenBucket};
use super::retry::{is_retryable, BackoffStrategy};

#[cfg(test)]
mod tests;

/// Configuration for the HTTP client.
#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    /// Request timeout. Default: 60 seconds.
    pub timeout: Duration,
    /// Maximum number of retry attempts. Default: 3.
    pub max_retries: u32,
    /// Backoff strategy for retries. Default: exponential (500ms base, 8s max).
    pub retry_backoff: BackoffStrategy,
    /// Optional rate limit configuration.
    pub rate_limit: Option<RateLimitConfig>,
    /// User-Agent header value.
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

/// Synchronous HTTP client for OCR API calls.
///
/// Wraps a `reqwest::blocking::Client` with authentication, retry logic,
/// and optional rate limiting. Designed for use in a thread-per-request model.
///
/// # Examples
///
/// ```ignore
/// use easypdf_ocr::http::{
///     OcrHttpClient, AuthMethod, HttpClientConfig,
/// };
///
/// let client = OcrHttpClient::new(
///     "https://api.example.com/ocr",
///     AuthMethod::Bearer("my-token".to_owned()),
/// );
/// ```
pub struct OcrHttpClient {
    client: reqwest::blocking::Client,
    endpoint: String,
    auth: AuthMethod,
    config: HttpClientConfig,
    rate_limiter: Option<TokenBucket>,
}

impl std::fmt::Debug for OcrHttpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OcrHttpClient")
            .field("endpoint", &self.endpoint)
            .field("auth", &self.auth)
            .field("config", &self.config)
            .field("rate_limiter", &self.rate_limiter)
            .finish_non_exhaustive()
    }
}

impl OcrHttpClient {
    /// Create a new HTTP client with default configuration.
    ///
    /// # Errors
    ///
    /// Returns `OcrHttpError::Transport` if the underlying HTTP client
    /// cannot be built.
    pub fn new(endpoint: impl Into<String>, auth: AuthMethod) -> Result<Self> {
        Self::with_config(endpoint, auth, HttpClientConfig::default())
    }

    /// Create a new HTTP client with custom configuration.
    ///
    /// # Errors
    ///
    /// Returns `OcrHttpError::Transport` if the underlying HTTP client
    /// cannot be built.
    pub fn with_config(
        endpoint: impl Into<String>,
        auth: AuthMethod,
        config: HttpClientConfig,
    ) -> Result<Self> {
        let rate_limiter = config
            .rate_limit
            .as_ref()
            .map(TokenBucket::new);

        let client = reqwest::blocking::Client::builder()
            .timeout(config.timeout)
            .user_agent(&config.user_agent)
            .build()
            .map_err(OcrHttpError::Transport)?;

        Ok(Self {
            client,
            endpoint: endpoint.into(),
            auth,
            config,
            rate_limiter,
        })
    }

    /// Get the endpoint URL.
    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Get a reference to the authentication method.
    #[must_use]
    pub fn auth(&self) -> &AuthMethod {
        &self.auth
    }

    /// POST a JSON body and deserialize the response.
    ///
    /// # Errors
    ///
    /// Returns `OcrHttpError` on transport, auth, or server errors.
    pub fn post_json<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        body: &T,
        extra_headers: Option<&HashMap<String, String>>,
    ) -> Result<R> {
        let auth_headers = apply_auth(&self.auth)?;
        let body_json = serde_json::to_vec(body)
            .map_err(|e| OcrHttpError::InvalidResponse(format!("JSON serialize error: {e}")))?;
        let response = self.execute(|client| {
            let mut req = client
                .post(&self.endpoint)
                .header("Content-Type", "application/json; charset=utf-8");

            // Apply authentication headers.
            for (key, value) in &auth_headers {
                // Skip pseudo-headers used internally by Tencent Cloud signing.
                if key.ends_with("-Pending") {
                    continue;
                }
                req = req.header(key.as_str(), value.as_str());
            }

            // Apply extra headers.
            if let Some(headers) = extra_headers {
                for (key, value) in headers {
                    req = req.header(key.as_str(), value.as_str());
                }
            }

            req.body(body_json.clone()).send()
        })?;

        response
            .json::<R>()
            .map_err(|e| OcrHttpError::InvalidResponse(format!("JSON parse error: {e}")))
    }

    /// POST multipart form data with an image file.
    ///
    /// # Errors
    ///
    /// Returns `OcrHttpError` on transport, auth, or server errors.
    ///
    /// # Panics
    ///
    /// Panics if the MIME type `"image/png"` is invalid (should not happen).
    pub fn post_multipart<R: for<'de> Deserialize<'de>>(
        &self,
        image: &easypdf_markdown::ocr::OcrImage,
        field_name: &str,
    ) -> Result<R> {
        let png_bytes = super::image::encode_to_png(image)?;
        let auth_headers = apply_auth(&self.auth)?;
        let fname = field_name.to_owned();

        let response = self.execute(|client| {
            let part = reqwest::blocking::multipart::Part::bytes(png_bytes.clone())
                .file_name(format!("{fname}.png"))
                .mime_str("image/png")
                .expect("valid mime type");

            let form = reqwest::blocking::multipart::Form::new().part(fname.clone(), part);

            let mut req = client.post(&self.endpoint).multipart(form);

            for (key, value) in &auth_headers {
                if key.ends_with("-Pending") {
                    continue;
                }
                req = req.header(key.as_str(), value.as_str());
            }

            req.send()
        })?;

        response
            .json::<R>()
            .map_err(|e| OcrHttpError::InvalidResponse(format!("JSON parse error: {e}")))
    }

    /// Execute a request builder with retry and rate limiting.
    ///
    /// This is the core method that handles:
    /// 1. Rate limiting (token bucket)
    /// 2. Request execution
    /// 3. Error classification (retryable vs non-retryable)
    /// 4. Exponential backoff between retries
    ///
    /// # Errors
    ///
    /// Returns `OcrHttpError::MaxRetriesExceeded` if all retries are exhausted.
    pub fn execute<F>(&self, request_builder: F) -> Result<reqwest::blocking::Response>
    where
        F: Fn(&reqwest::blocking::Client) -> reqwest::Result<reqwest::blocking::Response>,
    {
        let max_retries = self.config.max_retries;

        for attempt in 0..=max_retries {
            // Rate limit: wait for a token before each attempt.
            if let Some(ref limiter) = self.rate_limiter {
                limiter.acquire();
            }

            let result = (request_builder)(&self.client);

            match result {
                Ok(response) => {
                    let status = response.status().as_u16();

                    if response.status().is_success() {
                        return Ok(response);
                    }

                    // Check if this status is retryable.
                    if is_retryable(status) && attempt < max_retries {
                        let delay = self.config.retry_backoff.delay_for(attempt);
                        std::thread::sleep(delay);
                        continue;
                    }

                    // Non-retryable error or last attempt: extract error.
                    let body = response
                        .text()
                        .unwrap_or_else(|_| "<unreadable body>".to_owned());

                    return Err(match status {
                        429 => OcrHttpError::RateLimit {
                            retry_after_secs: 60,
                        },
                        400..=499 => OcrHttpError::BadRequest {
                            code: i32::from(status),
                            message: body,
                        },
                        _ => OcrHttpError::ServerError { status, body },
                    });
                }
                Err(e) => {
                    // Network errors are generally retryable.
                    if attempt < max_retries {
                        let delay = self.config.retry_backoff.delay_for(attempt);
                        std::thread::sleep(delay);
                        continue;
                    }
                    return Err(OcrHttpError::Transport(e));
                }
            }
        }

        Err(OcrHttpError::MaxRetriesExceeded { max: max_retries })
    }
}
