//! 智谱 GLM-OCR API 响应解析器。

use crate::http::error::{OcrHttpError, Result};
use crate::http::response::OcrResponseParser;
use easypdf_markdown::ocr::OcrResult;

/// 智谱 GLM-OCR 版面解析 API 响应解析器。
///
/// 将 GLM-OCR 端点的 JSON 响应解析为标准 [`OcrResult`]。
///
/// # API 响应结构
///
/// 根据官方文档
/// <https://docs.bigmodel.cn/cn/guide/models/vlm/glm-ocr>，响应
/// 预期遵循类似以下的结构：
///
/// ```json
/// {
///   "data": {
///     "text": "extracted text content...",
///     "pages": [...]
///   }
/// }
/// ```
///
/// 解析器还处理以下形式的错误响应：
///
/// ```json
/// {
///   "error": {
///     "code": "invalid_request",
///     "message": "..."
///   }
/// }
/// ```
///
/// **注意**：响应字段结构是根据智谱 `BigModel` API 惯例推断的。
/// 若实际响应不同，可能需要调整。
pub struct GlmOcrParser;

impl OcrResponseParser for GlmOcrParser {
    fn parse_response(&self, raw: &serde_json::Value) -> Result<OcrResult> {
        // 首先检查错误响应。
        if let Some(error) = raw.get("error") {
            let message = error
                .get("message")
                .and_then(|v| v.as_str())
                .or_else(|| error.get("msg").and_then(|v| v.as_str()))
                .unwrap_or("unknown error");
            let code = error
                .get("code")
                .and_then(serde_json::Value::as_i64)
                .map(|c| c.to_string())
                .or_else(|| error.get("code").and_then(|v| v.as_str()).map(String::from))
                .unwrap_or_else(|| "unknown".to_owned());
            return Err(OcrHttpError::Engine(format!(
                "GLM-OCR error (code: {code}): {message}"
            )));
        }

        // 从响应中提取文本。
        // 尝试多种响应结构模式：
        // 1. { "data": { "text": "..." } }
        // 2. { "text": "..." }
        // 3. { "choices": [{ "message": { "content": "..." } }] }（OpenAI 兼容格式）
        let text = extract_text(raw)?;

        if text.is_empty() {
            return Err(OcrHttpError::InvalidResponse(
                "GLM-OCR response contained no text content".to_owned(),
            ));
        }

        // 提取可选的置信度。
        let confidence = extract_confidence(raw);

        Ok(OcrResult {
            text,
            confidence,
            word_boxes: vec![],
        })
    }
}

/// 从多种可能的 GLM-OCR 响应结构中提取文本。
fn extract_text(raw: &serde_json::Value) -> Result<String> {
    // Pattern 1: { "data": { "text": "..." } }
    if let Some(text) = raw
        .get("data")
        .and_then(|d| d.get("text"))
        .and_then(|v| v.as_str())
    {
        return Ok(text.to_owned());
    }

    // Pattern 2: { "text": "..." }
    if let Some(text) = raw.get("text").and_then(|v| v.as_str()) {
        return Ok(text.to_owned());
    }

    // Pattern 3: { "choices": [{ "message": { "content": "..." } }] }
    // (OpenAI-compatible chat completion format)
    if let Some(content) = raw
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|msg| msg.get("content"))
        .and_then(|v| v.as_str())
    {
        return Ok(content.to_owned());
    }

    // Pattern 4: { "data": { "pages": [{ "text": "..." }] } }
    if let Some(pages) = raw
        .get("data")
        .and_then(|d| d.get("pages"))
        .and_then(|p| p.as_array())
    {
        let combined: String = pages
            .iter()
            .filter_map(|page| page.get("text").and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            .join("\n");
        if !combined.is_empty() {
            return Ok(combined);
        }
    }

    Err(OcrHttpError::InvalidResponse(
        "GLM-OCR response missing text field; expected 'data.text', 'text', \
         'choices[0].message.content', or 'data.pages[].text'"
            .to_owned(),
    ))
}

/// 从响应中提取可选的置信度分数。
#[allow(clippy::cast_possible_truncation)]
fn extract_confidence(raw: &serde_json::Value) -> Option<f32> {
    raw.get("confidence")
        .and_then(serde_json::Value::as_f64)
        .map(|f| f as f32)
        .or_else(|| {
            raw.get("data")
                .and_then(|d| d.get("confidence"))
                .and_then(serde_json::Value::as_f64)
                .map(|f| f as f32)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_success_data_text() {
        let raw = serde_json::json!({
            "data": {
                "text": "Hello, world!",
                "confidence": 0.95
            }
        });
        let parser = GlmOcrParser;
        let result = parser.parse_response(&raw).unwrap();
        assert_eq!(result.text, "Hello, world!");
        assert_eq!(result.confidence, Some(0.95));
    }

    #[test]
    fn test_parse_success_top_level_text() {
        let raw = serde_json::json!({
            "text": "Extracted content",
            "confidence": 0.88
        });
        let parser = GlmOcrParser;
        let result = parser.parse_response(&raw).unwrap();
        assert_eq!(result.text, "Extracted content");
        assert_eq!(result.confidence, Some(0.88));
    }

    #[test]
    fn test_parse_success_openai_compat() {
        let raw = serde_json::json!({
            "choices": [{
                "message": {
                    "content": "OCR result from chat format"
                }
            }]
        });
        let parser = GlmOcrParser;
        let result = parser.parse_response(&raw).unwrap();
        assert_eq!(result.text, "OCR result from chat format");
    }

    #[test]
    fn test_parse_success_pages() {
        let raw = serde_json::json!({
            "data": {
                "pages": [
                    { "text": "Page one." },
                    { "text": "Page two." }
                ]
            }
        });
        let parser = GlmOcrParser;
        let result = parser.parse_response(&raw).unwrap();
        assert_eq!(result.text, "Page one.\nPage two.");
    }

    #[test]
    fn test_parse_error_response() {
        let raw = serde_json::json!({
            "error": {
                "code": "invalid_request",
                "message": "File format not supported"
            }
        });
        let parser = GlmOcrParser;
        let err = parser.parse_response(&raw).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("File format not supported"));
    }

    #[test]
    fn test_parse_error_with_msg_field() {
        let raw = serde_json::json!({
            "error": {
                "code": 400,
                "msg": "Bad request"
            }
        });
        let parser = GlmOcrParser;
        let err = parser.parse_response(&raw).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Bad request"));
    }

    #[test]
    fn test_parse_empty_text() {
        let raw = serde_json::json!({
            "data": {
                "text": ""
            }
        });
        let parser = GlmOcrParser;
        let err = parser.parse_response(&raw).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("no text content"));
    }

    #[test]
    fn test_parse_missing_text_field() {
        let raw = serde_json::json!({
            "data": {
                "metadata": {}
            }
        });
        let parser = GlmOcrParser;
        let err = parser.parse_response(&raw).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("missing text field"));
    }

    #[test]
    fn test_confidence_extraction_from_data() {
        let raw = serde_json::json!({
            "data": {
                "text": "test",
                "confidence": 0.75
            }
        });
        let parser = GlmOcrParser;
        let result = parser.parse_response(&raw).unwrap();
        assert_eq!(result.confidence, Some(0.75));
    }

    #[test]
    fn test_no_confidence() {
        let raw = serde_json::json!({
            "data": {
                "text": "test"
            }
        });
        let parser = GlmOcrParser;
        let result = parser.parse_response(&raw).unwrap();
        assert!(result.confidence.is_none());
    }
}
