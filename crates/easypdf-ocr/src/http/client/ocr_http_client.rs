//! 同步 HTTP 客户端，用于 OCR API 调用。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::super::auth::{AuthMethod, apply_auth};
use super::super::error::{OcrHttpError, Result};
use super::super::rate_limit::TokenBucket;
use super::super::retry::is_retryable;
use super::http_client_config::HttpClientConfig;

/// 同步 HTTP 客户端，用于 OCR API 调用。
///
/// 封装 `reqwest::blocking::Client`，内置认证、重试逻辑和可选限流。
/// 适用于线程-per-请求模型。
///
/// # 线程安全
///
/// `OcrHttpClient` 是 `Send + Sync` 的，可跨线程共享。
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
    /// 底层 reqwest 阻塞客户端。
    client: reqwest::blocking::Client,
    /// API 端点 URL。
    endpoint: String,
    /// 认证方式。
    auth: AuthMethod,
    /// 客户端配置。
    config: HttpClientConfig,
    /// 可选的令牌桶限流器。
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
    /// 使用默认配置创建 HTTP 客户端。
    ///
    /// # 参数
    ///
    /// * `endpoint` - API 端点 URL。
    /// * `auth` - 认证方式。
    ///
    /// # Errors
    ///
    /// 若底层 HTTP 客户端构建失败，返回 `OcrHttpError::Transport`。
    pub fn new(endpoint: impl Into<String>, auth: AuthMethod) -> Result<Self> {
        Self::with_config(endpoint, auth, HttpClientConfig::default())
    }

    /// 使用自定义配置创建 HTTP 客户端。
    ///
    /// # 参数
    ///
    /// * `endpoint` - API 端点 URL。
    /// * `auth` - 认证方式。
    /// * `config` - 客户端配置（超时、重试、限流等）。
    ///
    /// # Errors
    ///
    /// 若底层 HTTP 客户端构建失败，返回 `OcrHttpError::Transport`。
    pub fn with_config(
        endpoint: impl Into<String>,
        auth: AuthMethod,
        config: HttpClientConfig,
    ) -> Result<Self> {
        let rate_limiter = config.rate_limit.as_ref().map(TokenBucket::new);

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

    /// 获取端点 URL。
    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// 获取认证方式的引用。
    #[must_use]
    pub fn auth(&self) -> &AuthMethod {
        &self.auth
    }

    /// 发送 JSON POST 请求并反序列化响应。
    ///
    /// # 参数
    ///
    /// * `body` - 要序列化为 JSON 的请求体。
    /// * `extra_headers` - 可选的额外请求头。
    ///
    /// # Errors
    ///
    /// 在传输、认证或服务器错误时返回 `OcrHttpError`。
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

            // 应用认证头。
            for (key, value) in &auth_headers {
                // 跳过腾讯云签名内部使用的伪头。
                if key.ends_with("-Pending") {
                    continue;
                }
                req = req.header(key.as_str(), value.as_str());
            }

            // 应用额外请求头。
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

    /// 发送带图像文件的 multipart/form-data POST 请求。
    ///
    /// # 参数
    ///
    /// * `image` - OCR 图像数据。
    /// * `field_name` - 表单中图像文件的字段名。
    ///
    /// # Errors
    ///
    /// 在传输、认证或服务器错误时返回 `OcrHttpError`。
    ///
    /// # Panics
    ///
    /// 若 MIME 类型 `"image/png"` 无效则 panic（正常情况下不会发生）。
    pub fn post_multipart<R: for<'de> Deserialize<'de>>(
        &self,
        image: &easypdf_markdown::ocr::OcrImage,
        field_name: &str,
    ) -> Result<R> {
        let png_bytes = super::super::image::encode_to_png(image)?;
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

    /// 执行请求构建器，带重试和限流。
    ///
    /// 这是核心方法，处理：
    /// 1. 限流（令牌桶）
    /// 2. 请求执行
    /// 3. 错误分类（可重试 vs 不可重试）
    /// 4. 重试间的指数退避
    ///
    /// # Errors
    ///
    /// 若所有重试耗尽，返回 `OcrHttpError::MaxRetriesExceeded`。
    pub fn execute<F>(&self, request_builder: F) -> Result<reqwest::blocking::Response>
    where
        F: Fn(&reqwest::blocking::Client) -> reqwest::Result<reqwest::blocking::Response>,
    {
        let max_retries = self.config.max_retries;

        for attempt in 0..=max_retries {
            // 限流：每次尝试前等待令牌。
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

                    // 检查该状态码是否可重试。
                    if is_retryable(status) && attempt < max_retries {
                        let delay = self.config.retry_backoff.delay_for(attempt);
                        std::thread::sleep(delay);
                        continue;
                    }

                    // 不可重试错误或最后一次尝试：提取错误。
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
                    // 网络错误通常可重试。
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
