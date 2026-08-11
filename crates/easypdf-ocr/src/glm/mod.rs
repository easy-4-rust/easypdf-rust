//! Zhipu `BigModel` GLM-OCR engine integration for easypdf.
//!
//! Provides an OCR engine backed by Zhipu AI's GLM-OCR model,
//! accessible via the `BigModel` platform layout parsing API.

pub mod config;
pub mod parser;
pub mod request;

use crate::http::error::Result;
use crate::http::{build_http_engine, HttpOcrEngine};

pub use config::{GlmConfig, GlmOutputFormat};
pub use parser::GlmOcrParser;
pub use request::GlmOcrRequest;

/// Create a GLM-OCR engine with the given configuration.
///
/// This is the primary entry point for constructing a GLM-OCR engine.
/// It builds an [`HttpOcrEngine`] that uses the Zhipu `BigModel` layout
/// parsing API for OCR.
///
/// # Errors
///
/// Returns `OcrHttpError::Transport` if the underlying HTTP client
/// cannot be initialized.
pub fn create_glm_ocr_engine(
    config: GlmConfig,
) -> Result<HttpOcrEngine<GlmOcrRequest, GlmOcrParser>> {
    let request = GlmOcrRequest::new(config);
    build_http_engine(request, GlmOcrParser)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::default_trait_access)]
    use super::*;
    use easypdf_markdown::ocr::{OcrEngine, OcrImage};
    use crate::http::request::OcrRequest;
    use crate::http::response::OcrResponseParser;

    fn make_test_image() -> OcrImage {
        let pixels = vec![255u8; 4 * 4 * 4]; // 4x4 RGBA
        OcrImage::new(4, 4, pixels)
    }

    #[test]
    fn test_create_engine_default() {
        let config = GlmConfig {
            api_key: "test-key".to_owned(),
            ..GlmConfig::default()
        };
        let engine = create_glm_ocr_engine(config).unwrap();
        assert_eq!(engine.name(), "glm-ocr");
    }

    #[test]
    fn test_engine_languages() {
        let config = GlmConfig {
            api_key: "test-key".to_owned(),
            ..GlmConfig::default()
        };
        let engine = create_glm_ocr_engine(config).unwrap();
        let langs = engine.languages();
        assert!(langs.contains(&"zh"));
        assert!(langs.contains(&"en"));
    }

    #[test]
    fn test_request_build_body_structure() {
        let config = GlmConfig {
            api_key: "test-key".to_owned(),
            language: Some("zh".to_owned()),
            ..GlmConfig::default()
        };
        let request = GlmOcrRequest::new(config);
        let image = make_test_image();
        let body = request
            .build_request_body(&image, &Default::default())
            .unwrap();

        // Verify JSON structure.
        assert_eq!(body["model"], "glm-ocr");
        let file_val = body["file"].as_str().unwrap();
        assert!(file_val.starts_with("data:image/png;base64,"));
        assert!(file_val.len() > 22); // "data:image/png;base64," + actual data
        assert_eq!(body["language"], "zh");
        assert_eq!(body["output_format"], "text");
    }

    #[test]
    fn test_parser_success() {
        let raw = serde_json::json!({
            "data": {
                "text": "Extracted OCR text",
                "confidence": 0.92
            }
        });
        let result = GlmOcrParser.parse_response(&raw).unwrap();
        assert_eq!(result.text, "Extracted OCR text");
        assert_eq!(result.confidence, Some(0.92));
    }

    #[test]
    fn test_parser_error() {
        let raw = serde_json::json!({
            "error": {
                "code": "rate_limit",
                "message": "Too many requests"
            }
        });
        let err = GlmOcrParser.parse_response(&raw).unwrap_err();
        assert!(err.to_string().contains("Too many requests"));
    }

    #[test]
    fn test_engine_debug_no_api_key() {
        let config = GlmConfig {
            api_key: "super-secret-api-key-12345".to_owned(),
            ..GlmConfig::default()
        };
        let engine = create_glm_ocr_engine(config).unwrap();
        let debug = format!("{engine:?}");
        assert!(debug.contains("glm-ocr"));
        assert!(!debug.contains("super-secret-api-key-12345"));
    }

    #[test]
    fn test_parser_openai_compat_format() {
        let raw = serde_json::json!({
            "choices": [{
                "message": {
                    "content": "Text from OpenAI-compat response"
                }
            }]
        });
        let result = GlmOcrParser.parse_response(&raw).unwrap();
        assert_eq!(result.text, "Text from OpenAI-compat response");
    }
}
