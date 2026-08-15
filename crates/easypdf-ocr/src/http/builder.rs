//! HTTP OCR 引擎工厂函数。

use super::client::HttpClientConfig;
use super::error::Result;
use super::http_ocr_engine::HttpOcrEngine;
use super::request::{OcrRequest, RequestConfig};
use super::response::OcrResponseParser;

/// 使用默认配置构建 HTTP OCR 引擎。
///
/// 这是创建基于 HTTP 的 OCR 引擎的最简方式。
///
/// # 参数
///
/// * `request` - 请求构建器，负责构造引擎专用的 JSON 请求体。
/// * `parser` - 响应解析器，负责从引擎专用 JSON 响应中提取识别结果。
///
/// # Errors
///
/// 若底层 HTTP 客户端构建失败，返回 `OcrHttpError::Transport`。
pub fn build_http_engine<R, P>(request: R, parser: P) -> Result<HttpOcrEngine<R, P>>
where
    R: OcrRequest,
    P: OcrResponseParser,
{
    HttpOcrEngine::new(request, parser)
}

/// 使用自定义配置构建 HTTP OCR 引擎。
///
/// # 参数
///
/// * `request` - 请求构建器。
/// * `parser` - 响应解析器。
/// * `config` - HTTP 客户端配置（超时、重试、限流等）。
///
/// # Errors
///
/// 若底层 HTTP 客户端构建失败，返回 `OcrHttpError::Transport`。
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
