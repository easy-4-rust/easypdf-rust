//! Request builder for the Zhipu GLM-OCR API.

use easypdf_markdown::ocr::OcrImage;
use crate::http::auth::AuthMethod;
use crate::http::error::Result;
use crate::http::image::{encode_for_request, ImageEncoding};
use crate::http::request::{OcrRequest, RequestConfig};

use super::config::{GlmConfig, GlmOutputFormat};

/// Request builder for the Zhipu GLM-OCR layout parsing API.
///
/// Constructs the JSON request body according to the GLM-OCR API specification.
///
/// # API Note
///
/// The request structure is based on the official documentation at
/// <https://docs.bigmodel.cn/cn/guide/models/vlm/glm-ocr>. The endpoint
/// is `POST /api/paas/v4/layout_parsing` with a JSON body containing
/// `model` and `file` (URL or data-URL) fields.
pub struct GlmOcrRequest {
    config: GlmConfig,
    auth: AuthMethod,
}

impl GlmOcrRequest {
    /// Create a new GLM-OCR request builder.
    ///
    /// # Arguments
    ///
    /// * `config` - GLM-OCR configuration including API key and endpoint.
    #[must_use]
    pub fn new(config: GlmConfig) -> Self {
        let auth = AuthMethod::Bearer(config.api_key.clone());
        Self { config, auth }
    }
}

impl OcrRequest for GlmOcrRequest {
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
        // Encode the image as base64 PNG and wrap in a data URL.
        let encoded = encode_for_request(image, &ImageEncoding::Base64Inline)?;
        let data_url = format!(
            "data:image/png;base64,{}",
            encoded.base64.as_deref().unwrap_or_default()
        );

        let mut body = serde_json::json!({
            "model": self.config.model,
            "file": data_url,
        });

        // Add optional language hint.
        if let Some(ref lang) = self.config.language {
            body["language"] = serde_json::Value::String(lang.clone());
        }

        // Add output format hint.
        match self.config.output_format {
            GlmOutputFormat::Text => {
                body["output_format"] = serde_json::Value::String("text".to_owned());
            }
            GlmOutputFormat::TextWithBoxes => {
                body["output_format"] =
                    serde_json::Value::String("text_with_boxes".to_owned());
            }
        }

        Ok(body)
    }

    fn engine_name(&self) -> &'static str {
        "glm-ocr"
    }

    fn languages(&self) -> &[&str] {
        &[
            "zh", "en", "fr", "es", "ru", "de", "ja", "ko",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_image() -> OcrImage {
        let pixels = vec![255u8; 4 * 4 * 4]; // 4x4 RGBA
        OcrImage::new(4, 4, pixels)
    }

    fn make_config() -> GlmConfig {
        GlmConfig {
            api_key: "test-key".to_owned(),
            ..GlmConfig::default()
        }
    }

    #[test]
    fn test_engine_name() {
        let req = GlmOcrRequest::new(make_config());
        assert_eq!(req.engine_name(), "glm-ocr");
    }

    #[test]
    fn test_endpoint() {
        let req = GlmOcrRequest::new(make_config());
        assert_eq!(
            req.endpoint(),
            "https://open.bigmodel.cn/api/paas/v4/layout_parsing"
        );
    }

    #[test]
    fn test_languages() {
        let req = GlmOcrRequest::new(make_config());
        let langs = req.languages();
        assert!(langs.contains(&"zh"));
        assert!(langs.contains(&"en"));
        assert!(langs.contains(&"ja"));
    }

    #[test]
    fn test_auth_is_bearer() {
        let req = GlmOcrRequest::new(make_config());
        let debug = format!("{:?}", req.auth());
        assert!(debug.contains("Bearer"));
        assert!(debug.contains("***"));
    }

    #[test]
    fn test_build_request_body_default() {
        let req = GlmOcrRequest::new(make_config());
        let image = make_test_image();
        let body = req
            .build_request_body(&image, &RequestConfig::default())
            .unwrap();

        assert_eq!(body["model"], "glm-ocr");
        assert!(body["file"].as_str().unwrap().starts_with("data:image/png;base64,"));
        assert_eq!(body["output_format"], "text");
        assert!(body.get("language").is_none());
    }

    #[test]
    fn test_build_request_body_with_language() {
        let mut config = make_config();
        config.language = Some("zh".to_owned());
        let req = GlmOcrRequest::new(config);
        let image = make_test_image();
        let body = req
            .build_request_body(&image, &RequestConfig::default())
            .unwrap();

        assert_eq!(body["language"], "zh");
    }

    #[test]
    fn test_build_request_body_with_boxes() {
        let mut config = make_config();
        config.output_format = GlmOutputFormat::TextWithBoxes;
        let req = GlmOcrRequest::new(config);
        let image = make_test_image();
        let body = req
            .build_request_body(&image, &RequestConfig::default())
            .unwrap();

        assert_eq!(body["output_format"], "text_with_boxes");
    }

    #[test]
    fn test_extra_headers_empty() {
        let req = GlmOcrRequest::new(make_config());
        assert!(req.extra_headers().is_empty());
    }
}
