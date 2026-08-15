//! Cloud OCR engine collection for easypdf.
//!
//! Provides HTTP-based OCR engines for GLM, `HunyuanOCR`, and Baidu Cloud.

#![deny(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::uninlined_format_args)]

pub mod baidu;
pub mod glm;
pub mod http;
pub mod hunyuan;

// Flat re-exports for convenience.
pub use http::auth::AuthMethod;
pub use http::client::HttpClientConfig;
pub use http::error::OcrHttpError;
pub use http::image::{EncodedImage, ImageEncoding};
pub use http::rate_limit::RateLimitConfig;
pub use http::request::{OcrRequest, RequestConfig};
pub use http::response::OcrResponseParser;
pub use http::retry::BackoffStrategy;
pub use http::{HttpOcrEngine, build_http_engine, build_http_engine_with_config};

pub use baidu::{
    BaiduApi, BaiduConfig, BaiduError, BaiduOcrEngine, BaiduOcrParser, BaiduResult, TokenManager,
};
pub use glm::{GlmConfig, GlmOcrParser, GlmOcrRequest, GlmOutputFormat, create_glm_ocr_engine};
pub use hunyuan::{
    HunyuanConfig, HunyuanMode, HunyuanOcrParser, HunyuanOcrRequest, create_hunyuan_ocr_engine,
};
