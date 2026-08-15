//! GLM-OCR 引擎工厂函数。

use crate::http::builder::build_http_engine;
use crate::http::error::Result;
use crate::http::http_ocr_engine::HttpOcrEngine;

use super::config::GlmConfig;
use super::parser::GlmOcrParser;
use super::request::GlmOcrRequest;

/// 使用给定配置创建 GLM-OCR 引擎。
///
/// 这是构建 GLM-OCR 引擎的主要入口。它构建一个使用智谱 `BigModel`
/// 版面解析 API 的 [`HttpOcrEngine`]。
///
/// # 参数
///
/// * `config` - GLM-OCR 配置，包含 API 密钥和端点等信息。
///
/// # Errors
///
/// 若底层 HTTP 客户端初始化失败，返回 `OcrHttpError::Transport`。
pub fn create_glm_ocr_engine(
    config: GlmConfig,
) -> Result<HttpOcrEngine<GlmOcrRequest, GlmOcrParser>> {
    let request = GlmOcrRequest::new(config);
    build_http_engine(request, GlmOcrParser)
}
