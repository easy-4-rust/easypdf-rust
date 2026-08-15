//! 基于 HTTP 的 OCR 引擎响应解析 trait。
//!
//! 每个 OCR 引擎实现 [`OcrResponseParser`] 来描述如何从其专用
//! JSON 响应中提取文本、置信度和词框。

use easypdf_markdown::ocr::OcrResult;

use super::error::Result;

/// 解析 OCR 引擎 JSON 响应的 trait。
///
/// 每个云 OCR 提供商以不同格式返回结果。实现者将原始 JSON
/// 解析为标准化的 [`OcrResult`]。
///
/// # 实现示例
///
/// ```ignore
/// use easypdf_ocr::http::{
///     OcrResponseParser,
///     error::Result,
/// };
/// use easypdf_markdown::ocr::OcrResult;
///
/// struct GlmOcrParser;
///
/// impl OcrResponseParser for GlmOcrParser {
///     fn parse_response(&self, raw: &serde_json::Value) -> Result<OcrResult> {
///         // Extract text from engine-specific JSON structure
///         let text = raw["data"]["text"].as_str().unwrap_or("");
///         Ok(OcrResult { text: text.to_string(), ..Default::default() })
///     }
/// }
/// ```
pub trait OcrResponseParser: Send + Sync {
    /// 将原始 JSON 响应解析为 [`OcrResult`]。
    ///
    /// # Errors
    ///
    /// 若响应结构不符合预期，返回 `OcrHttpError::InvalidResponse`；
    /// 若引擎返回了应用级错误，返回 `OcrHttpError::Engine`。
    fn parse_response(&self, raw: &serde_json::Value) -> Result<OcrResult>;
}
