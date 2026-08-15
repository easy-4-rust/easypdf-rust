//! 智谱 `BigModel` GLM-OCR 引擎集成。
//!
//! 提供基于智谱 AI 的 GLM-OCR 模型的 OCR 引擎，
//! 通过 `BigModel` 平台版面解析 API 访问。

pub mod config;
pub mod factory;
pub mod parser;
pub mod request;

pub use config::{GlmConfig, GlmOutputFormat};
pub use factory::create_glm_ocr_engine;
pub use parser::GlmOcrParser;
pub use request::GlmOcrRequest;

#[cfg(test)]
mod tests {
    #![allow(clippy::default_trait_access)]
    use super::*;
    use crate::http::request::OcrRequest;
    use crate::http::response::OcrResponseParser;
    use easypdf_markdown::ocr::{OcrEngine, OcrImage};

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

        // 验证 JSON 结构。
        assert_eq!(body["model"], "glm-ocr");
        let file_val = body["file"].as_str().unwrap();
        assert!(file_val.starts_with("data:image/png;base64,"));
        assert!(file_val.len() > 22); // "data:image/png;base64," + 实际数据
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
