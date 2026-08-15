//! 腾讯云 OCR 引擎集成。
//!
//! 提供基于腾讯云 OCR API 的引擎，支持三种识别模式。

pub mod config;
pub mod factory;
pub mod parser;
pub mod request;

pub use config::{HunyuanConfig, HunyuanMode};
pub use factory::create_hunyuan_ocr_engine;
pub use parser::HunyuanOcrParser;
pub use request::HunyuanOcrRequest;

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
