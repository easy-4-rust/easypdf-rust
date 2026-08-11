//! Request builder for Tencent Cloud OCR API.
//!
//! Implements [`OcrRequest`] for the Tencent Cloud OCR endpoints,
//! supporting `GeneralBasicOCR`, `SmartStructuralOCR`, and
//! `GeneralAccurateOCR` actions.

use std::collections::HashMap;

use easypdf_markdown::ocr::OcrImage;
use crate::http::auth::AuthMethod;
use crate::http::error::Result;
use crate::http::image::{encode_for_request, ImageEncoding};
use crate::http::request::{OcrRequest, RequestConfig};

use super::config::{HunyuanConfig, HunyuanMode};

/// Request builder for Tencent Cloud OCR API.
///
/// Constructs the JSON request body and extra headers for the selected
/// OCR mode. Uses [`AuthMethod::TencentCloud`] for TC3-HMAC-SHA256
/// signature authentication.
///
/// # API Actions
///
/// | Mode | Action |
/// |------|--------|
/// | `GeneralBasic` | `GeneralBasicOCR` |
/// | `SmartStructural` | `SmartStructuralOCR` |
/// | `GeneralAccurate` | `GeneralAccurateOCR` |
pub struct HunyuanOcrRequest {
    config: HunyuanConfig,
    auth: AuthMethod,
}

impl HunyuanOcrRequest {
    /// Create a new request builder from configuration.
    ///
    /// Constructs the [`AuthMethod::TencentCloud`] variant with the
    /// provided credentials.
    #[must_use]
    pub fn new(config: HunyuanConfig) -> Self {
        let auth = AuthMethod::TencentCloud {
            secret_id: config.secret_id.clone(),
            secret_key: config.secret_key.clone(),
            service: config.service.clone(),
            host: "ocr.tencentcloudapi.com".to_string(),
            region: config.region.clone(),
            version: config.version.clone(),
        };
        Self { config, auth }
    }

    /// Get a reference to the configuration.
    #[must_use]
    pub fn config(&self) -> &HunyuanConfig {
        &self.config
    }
}

impl OcrRequest for HunyuanOcrRequest {
    fn endpoint(&self) -> &str {
        &self.config.endpoint
    }

    fn auth(&self) -> &AuthMethod {
        &self.auth
    }

    fn build_request_body(
        &self,
        image: &OcrImage,
        _config: &RequestConfig,
    ) -> Result<serde_json::Value> {
        let encoded = encode_for_request(image, &ImageEncoding::Base64Inline)?;
        let base64_data = encoded.base64.unwrap_or_default();

        let mut body = serde_json::json!({
            "ImageBase64": base64_data,
        });

        // Add optional language hint.
        if let Some(ref lang) = self.config.language {
            body["Language"] = serde_json::Value::String(lang.clone());
        }

        // For SmartStructuralOCR, enable full text return by default.
        if self.config.mode == HunyuanMode::SmartStructural {
            body["ReturnFullText"] = serde_json::Value::Bool(true);
        }

        Ok(body)
    }

    fn extra_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert(
            "X-TC-Action".to_string(),
            self.config.mode.action_name().to_string(),
        );
        headers
    }

    fn engine_name(&self) -> &'static str {
        self.config.mode.engine_name()
    }

    fn languages(&self) -> &[&str] {
        &["zh", "en", "ja", "ko", "auto"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_image() -> OcrImage {
        let pixels = vec![255u8; 4 * 2 * 2]; // 2x2 RGBA
        OcrImage::new(2, 2, pixels)
    }

    fn make_config(mode: HunyuanMode) -> HunyuanConfig {
        HunyuanConfig {
            secret_id: "test-id".to_string(),
            secret_key: "test-key".to_string(),
            mode,
            ..HunyuanConfig::default()
        }
    }

    #[test]
    fn test_build_request_body_has_image_base64() {
        let request = HunyuanOcrRequest::new(make_config(HunyuanMode::GeneralBasic));
        let image = make_test_image();
        let body = request
            .build_request_body(&image, &RequestConfig::default())
            .unwrap();
        assert!(
            body.get("ImageBase64").is_some(),
            "body must contain ImageBase64"
        );
        assert!(
            !body["ImageBase64"].as_str().unwrap().is_empty(),
            "ImageBase64 must not be empty"
        );
    }

    #[test]
    fn test_build_request_body_with_language() {
        let mut config = make_config(HunyuanMode::GeneralBasic);
        config.language = Some("en".to_string());
        let request = HunyuanOcrRequest::new(config);
        let image = make_test_image();
        let body = request
            .build_request_body(&image, &RequestConfig::default())
            .unwrap();
        assert_eq!(body["Language"].as_str().unwrap(), "en");
    }

    #[test]
    fn test_build_request_body_without_language() {
        let request = HunyuanOcrRequest::new(make_config(HunyuanMode::GeneralBasic));
        let image = make_test_image();
        let body = request
            .build_request_body(&image, &RequestConfig::default())
            .unwrap();
        assert!(
            body.get("Language").is_none(),
            "Language should be absent when not configured"
        );
    }

    #[test]
    fn test_build_request_body_smart_structural_has_return_full_text() {
        let request = HunyuanOcrRequest::new(make_config(HunyuanMode::SmartStructural));
        let image = make_test_image();
        let body = request
            .build_request_body(&image, &RequestConfig::default())
            .unwrap();
        assert!(body["ReturnFullText"].as_bool().unwrap());
    }

    #[test]
    fn test_build_request_body_general_basic_no_return_full_text() {
        let request = HunyuanOcrRequest::new(make_config(HunyuanMode::GeneralBasic));
        let image = make_test_image();
        let body = request
            .build_request_body(&image, &RequestConfig::default())
            .unwrap();
        assert!(
            body.get("ReturnFullText").is_none(),
            "ReturnFullText should be absent for GeneralBasic"
        );
    }

    #[test]
    fn test_extra_headers_contain_tc_action_general_basic() {
        let request = HunyuanOcrRequest::new(make_config(HunyuanMode::GeneralBasic));
        let headers = request.extra_headers();
        assert_eq!(headers.get("X-TC-Action").unwrap(), "GeneralBasicOCR");
    }

    #[test]
    fn test_extra_headers_contain_tc_action_smart_structural() {
        let request = HunyuanOcrRequest::new(make_config(HunyuanMode::SmartStructural));
        let headers = request.extra_headers();
        assert_eq!(headers.get("X-TC-Action").unwrap(), "SmartStructuralOCR");
    }

    #[test]
    fn test_extra_headers_contain_tc_action_general_accurate() {
        let request = HunyuanOcrRequest::new(make_config(HunyuanMode::GeneralAccurate));
        let headers = request.extra_headers();
        assert_eq!(headers.get("X-TC-Action").unwrap(), "GeneralAccurateOCR");
    }

    #[test]
    fn test_engine_name_general_basic() {
        let request = HunyuanOcrRequest::new(make_config(HunyuanMode::GeneralBasic));
        assert_eq!(request.engine_name(), "hunyuan-general-basic");
    }

    #[test]
    fn test_engine_name_smart_structural() {
        let request = HunyuanOcrRequest::new(make_config(HunyuanMode::SmartStructural));
        assert_eq!(request.engine_name(), "hunyuan-smart-structural");
    }

    #[test]
    fn test_engine_name_general_accurate() {
        let request = HunyuanOcrRequest::new(make_config(HunyuanMode::GeneralAccurate));
        assert_eq!(request.engine_name(), "hunyuan-general-accurate");
    }

    #[test]
    fn test_endpoint() {
        let request = HunyuanOcrRequest::new(make_config(HunyuanMode::GeneralBasic));
        assert_eq!(request.endpoint(), "https://ocr.tencentcloudapi.com");
    }

    #[test]
    fn test_languages() {
        let request = HunyuanOcrRequest::new(make_config(HunyuanMode::GeneralBasic));
        assert_eq!(request.languages(), &["zh", "en", "ja", "ko", "auto"]);
    }

    #[test]
    fn test_auth_is_tencent_cloud() {
        let request = HunyuanOcrRequest::new(make_config(HunyuanMode::GeneralBasic));
        match request.auth() {
            AuthMethod::TencentCloud {
                secret_id,
                service,
                host,
                region,
                version,
                ..
            } => {
                assert_eq!(secret_id, "test-id");
                assert_eq!(service, "ocr");
                assert_eq!(host, "ocr.tencentcloudapi.com");
                assert_eq!(region, "ap-guangzhou");
                assert_eq!(version, "2018-11-19");
            }
            other => panic!("expected TencentCloud auth, got {other:?}"),
        }
    }
}
