//! Tencent Cloud OCR engine integration for easypdf.
//!
//! Provides an OCR engine backed by Tencent Cloud's OCR API with three
//! recognition modes.

pub mod config;
pub mod parser;
pub mod request;

pub use config::{HunyuanConfig, HunyuanMode};
pub use parser::HunyuanOcrParser;
pub use request::HunyuanOcrRequest;

use crate::http::build_http_engine;
use crate::http::error::Result;
use crate::http::HttpOcrEngine;

/// Create a Tencent Cloud OCR engine with default HTTP configuration.
///
/// # Errors
///
/// Returns [`OcrHttpError::Transport`](crate::http::OcrHttpError::Transport)
/// if the underlying HTTP client cannot be built.
pub fn create_hunyuan_ocr_engine(
    config: HunyuanConfig,
) -> Result<HttpOcrEngine<HunyuanOcrRequest, HunyuanOcrParser>> {
    let mode = config.mode;
    build_http_engine(HunyuanOcrRequest::new(config), HunyuanOcrParser::new(mode))
}

#[cfg(test)]
mod tests {
    use super::*;
    use easypdf_markdown::ocr::OcrEngine;

    #[test]
    fn test_create_engine_general_basic() {
        let config = HunyuanConfig {
            secret_id: "test-id".to_string(),
            secret_key: "test-key".to_string(),
            mode: HunyuanMode::GeneralBasic,
            ..HunyuanConfig::default()
        };
        let engine = create_hunyuan_ocr_engine(config).unwrap();
        assert_eq!(engine.name(), "hunyuan-general-basic");
        assert_eq!(engine.languages(), &["zh", "en", "ja", "ko", "auto"]);
    }

    #[test]
    fn test_create_engine_smart_structural() {
        let config = HunyuanConfig {
            secret_id: "test-id".to_string(),
            secret_key: "test-key".to_string(),
            mode: HunyuanMode::SmartStructural,
            ..HunyuanConfig::default()
        };
        let engine = create_hunyuan_ocr_engine(config).unwrap();
        assert_eq!(engine.name(), "hunyuan-smart-structural");
    }

    #[test]
    fn test_create_engine_general_accurate() {
        let config = HunyuanConfig {
            secret_id: "test-id".to_string(),
            secret_key: "test-key".to_string(),
            mode: HunyuanMode::GeneralAccurate,
            ..HunyuanConfig::default()
        };
        let engine = create_hunyuan_ocr_engine(config).unwrap();
        assert_eq!(engine.name(), "hunyuan-general-accurate");
    }

    #[test]
    fn test_engine_debug_redacts_credentials() {
        let config = HunyuanConfig {
            secret_id: "AKID1234567890".to_string(),
            secret_key: "super-secret-key".to_string(),
            mode: HunyuanMode::GeneralBasic,
            ..HunyuanConfig::default()
        };
        let engine = create_hunyuan_ocr_engine(config).unwrap();
        let debug = format!("{engine:?}");
        assert!(!debug.contains("super-secret-key"));
        assert!(!debug.contains("AKID1234567890"));
    }
}
