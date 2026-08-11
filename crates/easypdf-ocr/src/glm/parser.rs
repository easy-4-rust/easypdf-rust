//! Response parser for the Zhipu GLM-OCR API.

use easypdf_markdown::ocr::OcrResult;
use crate::http::error::{OcrHttpError, Result};
use crate::http::response::OcrResponseParser;

/// Response parser for the Zhipu GLM-OCR layout parsing API.
///
/// Parses the JSON response from the GLM-OCR endpoint into a standard
/// [`OcrResult`].
///
/// # API Response Structure
///
/// Based on the official documentation at
/// <https://docs.bigmodel.cn/cn/guide/models/vlm/glm-ocr>, the response
/// is expected to follow a structure similar to:
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
/// The parser also handles error responses of the form:
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
/// **Note**: The exact response field structure is inferred from the
/// Zhipu `BigModel` API conventions. If the actual response differs,
/// adjustments may be needed.
pub struct GlmOcrParser;

impl OcrResponseParser for GlmOcrParser {
    fn parse_response(&self, raw: &serde_json::Value) -> Result<OcrResult> {
        // Check for error responses first.
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
                .or_else(|| {
                    error
                        .get("code")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                })
                .unwrap_or_else(|| "unknown".to_owned());
            return Err(OcrHttpError::Engine(format!(
                "GLM-OCR error (code: {code}): {message}"
            )));
        }

        // Extract text from the response.
        // Try multiple response structure patterns:
        // 1. { "data": { "text": "..." } }
        // 2. { "text": "..." }
        // 3. { "choices": [{ "message": { "content": "..." } }] } (OpenAI-compat)
        let text = extract_text(raw)?;

        if text.is_empty() {
            return Err(OcrHttpError::InvalidResponse(
                "GLM-OCR response contained no text content".to_owned(),
            ));
        }

        // Extract optional confidence.
        let confidence = extract_confidence(raw);

        Ok(OcrResult {
            text,
            confidence,
            word_boxes: vec![],
        })
    }
}

/// Extract text from various possible GLM-OCR response structures.
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

/// Extract optional confidence score from the response.
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
