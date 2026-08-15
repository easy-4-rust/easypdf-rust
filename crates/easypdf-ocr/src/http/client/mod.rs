//! OCR API 同步 HTTP 客户端。
//!
//! 提供 [`OcrHttpClient`]，一个内置重试、限流和认证的阻塞式 HTTP 客户端。
//! 使用 `reqwest::blocking::Client`，无需异步运行时。

pub mod http_client_config;
pub mod ocr_http_client;

#[cfg(test)]
mod tests;

pub use http_client_config::HttpClientConfig;
pub use ocr_http_client::OcrHttpClient;
