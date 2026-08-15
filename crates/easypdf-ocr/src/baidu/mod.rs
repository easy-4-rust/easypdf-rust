//! 百度云 OCR 引擎。
//!
//! 提供支持多种 API 端点的百度云 OCR 引擎，通过 [`BaiduApi`] 枚举选择。
//! 实现了 `easypdf-markdown-ocr` 的 [`OcrEngine`](easypdf_markdown::ocr::OcrEngine)，可直接用于 OCR 处理流水线。
//!
//! # 支持的 API
//!
//! | API | 变体 | 状态 |
//! |-----|------|------|
//! | 通用文字识别（标准版） | [`BaiduApi::GeneralBasic`] | 支持 |
//! | 通用文字识别（高精度版） | [`BaiduApi::GeneralAccurate`] | 支持 |
//! | 通用文字识别（标准含位置） | [`BaiduApi::GeneralBasicWithLocation`] | 支持 |
//! | 通用文字识别（高精度含位置） | [`BaiduApi::GeneralAccurateWithLocation`] | 支持 |
//! | 表格识别 V2 | [`BaiduApi::TableRecognitionV2`] | 支持 |
//! | 网络图片 | [`BaiduApi::WebImage`] | 支持 |
//! | 网络图片含位置 | [`BaiduApi::WebImageWithLocation`] | 支持 |
//! | 千帆 OCR | [`BaiduApi::QianfanOcr`] | 支持 |
//! | 办公文档 | [`BaiduApi::OfficeDocument`] | 支持 |
//! | 手写体 | [`BaiduApi::Handwriting`] | 支持 |
//! | 印章 | [`BaiduApi::Seal`] | 支持 |
//! | 数字 | [`BaiduApi::Digit`] | 支持 |
//! | 二维码 | [`BaiduApi::Qrcode`] | 支持 |
//! | 智能结构化 | [`BaiduApi::Structured`] | 支持 |
//! | 文档解析 | [`BaiduApi::DocParser`] | 桩（异步 API） |
//! | 文档解析（Paddle） | [`BaiduApi::DocParserPaddle`] | 桩（异步 API） |
//!
//! # 认证
//!
//! 百度 OCR 使用 OAuth 2.0 客户端凭证流程。引擎自动将 API Key + Secret Key
//! 交换为 `access_token` 并缓存。令牌有效期约 30 天，会自动刷新。
//!
//! # 请求格式
//!
//! 百度标准 OCR API 使用 `application/x-www-form-urlencoded`，图像以 base64
//! 编码放在 `image` 字段中。`access_token` 作为 URL 查询参数传递。
//!
//! 千帆 OCR 使用 JSON + Bearer 令牌认证。
//!
//! # 快速开始
//!
//! ```ignore
//! use easypdf_ocr::baidu::{BaiduOcrEngine, BaiduConfig, BaiduApi};
//!
//! let config = BaiduConfig {
//!     api_key: "your-api-key".to_owned(),
//!     secret_key: "your-secret-key".to_owned(),
//!     api: BaiduApi::GeneralAccurate,
//!     ..BaiduConfig::default()
//! };
//! let engine = BaiduOcrEngine::new(config);
//! // let result = engine.recognize(&image)?;
//! ```

pub mod baidu_ocr_engine;
pub mod config;
pub mod parser;
pub mod token;

pub use baidu_ocr_engine::BaiduOcrEngine;
pub use config::{BaiduApi, BaiduConfig, BaiduError, BaiduResult};
pub use parser::BaiduOcrParser;
pub use token::TokenManager;

#[cfg(test)]
mod tests {
    use super::*;
    use easypdf_markdown::ocr::{OcrEngine, OcrImage};

    fn make_test_image(width: u32, height: u32) -> OcrImage {
        let pixels = vec![255u8; (width * height * 4) as usize];
        OcrImage::new(width, height, pixels)
    }

    #[test]
    fn test_baidu_ocr_engine_debug_redacts_secret() {
        let config = BaiduConfig {
            api_key: "my-api-key".to_owned(),
            secret_key: "super-secret-key".to_owned(),
            api: BaiduApi::GeneralBasic,
            ..BaiduConfig::default()
        };
        let engine = BaiduOcrEngine::new(config);
        let debug = format!("{engine:?}");
        assert!(!debug.contains("super-secret-key"));
        assert!(debug.contains("my-api-key"));
        assert!(debug.contains("***"));
    }

    #[test]
    fn test_baidu_ocr_engine_name() {
        let config = BaiduConfig {
            api: BaiduApi::GeneralAccurate,
            ..BaiduConfig::default()
        };
        let engine = BaiduOcrEngine::new(config);
        assert_eq!(engine.name(), "baidu-general-accurate");
    }

    #[test]
    fn test_baidu_ocr_engine_languages() {
        let config = BaiduConfig::default();
        let engine = BaiduOcrEngine::new(config);
        assert!(engine.languages().contains(&"zh"));
        assert!(engine.languages().contains(&"en"));
    }

    #[test]
    fn test_baidu_ocr_engine_level() {
        let config = BaiduConfig::default();
        let engine = BaiduOcrEngine::new(config);
        assert_eq!(engine.level(), easypdf_core::CapabilityLevel::Cloud);
    }

    #[test]
    fn test_encode_to_png() {
        let image = make_test_image(2, 2);
        let png = baidu_ocr_engine::encode_to_png(&image).unwrap();
        assert!(png.starts_with(b"\x89PNG"));
    }

    #[test]
    fn test_encode_to_png_invalid() {
        let image = OcrImage::new(2, 2, vec![0u8; 10]); // 大小不匹配
        let result = baidu_ocr_engine::encode_to_png(&image);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_url() {
        let config = BaiduConfig {
            api: BaiduApi::GeneralAccurate,
            endpoint: "https://aip.baidubce.com/rest/2.0/ocr/v1".to_owned(),
            ..BaiduConfig::default()
        };
        let engine = BaiduOcrEngine::new(config);
        let url = engine.build_url("test-token-123");
        assert_eq!(
            url,
            "https://aip.baidubce.com/rest/2.0/ocr/v1/accurate_basic?access_token=test-token-123"
        );
    }

    #[test]
    fn test_engine_config_accessor() {
        let config = BaiduConfig {
            api: BaiduApi::TableRecognitionV2,
            api_key: "key123".to_owned(),
            ..BaiduConfig::default()
        };
        let engine = BaiduOcrEngine::new(config);
        assert_eq!(engine.config().api, BaiduApi::TableRecognitionV2);
        assert_eq!(engine.config().api_key, "key123");
    }

    #[test]
    fn test_encode_image_form() {
        use base64::Engine;
        let image = make_test_image(1, 1);
        let encoded = BaiduOcrEngine::encode_image_form(&image).unwrap();
        // 应为 URL 编码的 base64。
        assert!(!encoded.is_empty());
        // 验证解码后为有效的 base64。
        let decoded = urlencoding::decode(&encoded).unwrap();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(decoded.as_ref())
            .unwrap();
        assert!(bytes.starts_with(b"\x89PNG"));
    }
}
