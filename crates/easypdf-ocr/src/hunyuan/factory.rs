//! 腾讯云 OCR 引擎工厂函数。

use crate::http::builder::build_http_engine;
use crate::http::error::Result;
use crate::http::http_ocr_engine::HttpOcrEngine;

use super::config::HunyuanConfig;
use super::parser::HunyuanOcrParser;
use super::request::HunyuanOcrRequest;

/// 使用默认 HTTP 配置创建腾讯云 OCR 引擎。
///
/// # 参数
///
/// * `config` - 腾讯云 OCR 配置，包含密钥、区域和识别模式等信息。
///
/// # Errors
///
/// 若底层 HTTP 客户端构建失败，返回
/// [`OcrHttpError::Transport`](crate::http::OcrHttpError::Transport)。
pub fn create_hunyuan_ocr_engine(
    config: HunyuanConfig,
) -> Result<HttpOcrEngine<HunyuanOcrRequest, HunyuanOcrParser>> {
    let mode = config.mode;
    build_http_engine(HunyuanOcrRequest::new(config), HunyuanOcrParser::new(mode))
}
