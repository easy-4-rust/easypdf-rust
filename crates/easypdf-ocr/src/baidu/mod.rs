//! Baidu Cloud OCR engine for `easypdf-markdown`.
//!
//! This crate provides a Baidu Cloud OCR engine that supports multiple API
//! endpoints through the [`BaiduApi`] enum. It implements [`OcrEngine`] from
//! `easypdf-markdown-ocr` and can be used directly in the OCR processor pipeline.
//!
//! # Supported APIs
//!
//! | API | Variant | Status |
//! |-----|---------|--------|
//! | General (standard) | [`BaiduApi::GeneralBasic`] | Supported |
//! | General (high accuracy) | [`BaiduApi::GeneralAccurate`] | Supported |
//! | General (standard + location) | [`BaiduApi::GeneralBasicWithLocation`] | Supported |
//! | General (high accuracy + location) | [`BaiduApi::GeneralAccurateWithLocation`] | Supported |
//! | Table V2 | [`BaiduApi::TableRecognitionV2`] | Supported |
//! | Web image | [`BaiduApi::WebImage`] | Supported |
//! | Web image + location | [`BaiduApi::WebImageWithLocation`] | Supported |
//! | Qianfan-OCR | [`BaiduApi::QianfanOcr`] | Supported |
//! | Office document | [`BaiduApi::OfficeDocument`] | Supported |
//! | Handwriting | [`BaiduApi::Handwriting`] | Supported |
//! | Seal | [`BaiduApi::Seal`] | Supported |
//! | Digit | [`BaiduApi::Digit`] | Supported |
//! | QR code | [`BaiduApi::Qrcode`] | Supported |
//! | Structured | [`BaiduApi::Structured`] | Supported |
//! | Doc parser | [`BaiduApi::DocParser`] | Stub (async API) |
//! | Doc parser (Paddle) | [`BaiduApi::DocParserPaddle`] | Stub (async API) |
//!
//! # Authentication
//!
//! Baidu OCR uses OAuth 2.0 client-credentials flow. The engine automatically
//! exchanges API key + secret key for an access token and caches it. Tokens
//! are valid for ~30 days and are refreshed automatically.
//!
//! # Request Format
//!
//! Baidu standard OCR APIs use `application/x-www-form-urlencoded` with a
//! base64-encoded image in the `image` field. The `access_token` is passed
//! as a URL query parameter.
//!
//! Qianfan-OCR uses JSON with Bearer token authentication.
//!
//! # Quick Start
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

pub mod config;
pub mod parser;
pub mod token;

use base64::Engine;
use easypdf_core::CapabilityLevel;
use easypdf_markdown::ocr::{OcrEngine, OcrImage, OcrResult};

pub use config::{BaiduApi, BaiduConfig, BaiduError, BaiduResult};
pub use parser::BaiduOcrParser;
pub use token::TokenManager;

/// Baidu Cloud OCR engine.
///
/// Implements [`OcrEngine`] by performing OAuth token exchange (cached) and
/// sending form-urlencoded requests to the Baidu OCR API. Supports multiple
/// API endpoints via [`BaiduApi`].
///
/// # Thread Safety
///
/// `BaiduOcrEngine` is `Send + Sync` and can be shared across threads.
/// The OAuth token cache uses `parking_lot::Mutex` for contention-free access.
pub struct BaiduOcrEngine {
    /// Engine configuration.
    config: BaiduConfig,
    /// OAuth token manager (for standard APIs).
    token_manager: TokenManager,
    /// Response parser.
    parser: BaiduOcrParser,
    /// HTTP client (reused for connection pooling).
    client: reqwest::blocking::Client,
}

impl std::fmt::Debug for BaiduOcrEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BaiduOcrEngine")
            .field("api", &self.config.api)
            .field("endpoint", &self.config.endpoint)
            .field("api_key", &self.config.api_key)
            .field("secret_key", &"***")
            .field("token_manager", &self.token_manager)
            .field("parser", &self.parser)
            .finish_non_exhaustive()
    }
}

impl BaiduOcrEngine {
    /// Create a new Baidu OCR engine with the given configuration.
    ///
    /// # Panics
    ///
    /// Panics if the `reqwest` HTTP client cannot be built (should not happen
    /// in practice).
    #[must_use]
    pub fn new(config: BaiduConfig) -> Self {
        let token_manager = TokenManager::new(
            config.token_url.clone(),
            config.api_key.clone(),
            config.secret_key.clone(),
        );
        let parser = BaiduOcrParser::new(config.api);
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("failed to build HTTP client");

        Self {
            config,
            token_manager,
            parser,
            client,
        }
    }

    /// Build the full request URL for a standard Baidu OCR API.
    ///
    /// Format: `{endpoint}/{path}?access_token={token}`
    fn build_url(&self, token: &str) -> String {
        format!(
            "{}/{}?access_token={}",
            self.config.endpoint,
            self.config.api.path(),
            token
        )
    }

    /// Build the request URL for Qianfan-OCR.
    fn build_qianfan_url(&self) -> &str {
        &self.config.qianfan_endpoint
    }

    /// Encode an image to base64 and URL-encode it for form submission.
    fn encode_image_form(image: &OcrImage) -> BaiduResult<String> {
        let png_bytes = encode_to_png(image)?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
        Ok(urlencoding::encode(&b64).into_owned())
    }

    /// Execute a standard Baidu OCR request (form-urlencoded with `access_token`).
    fn execute_standard(&self, image: &OcrImage) -> BaiduResult<OcrResult> {
        if !self.config.api.is_supported() {
            return Err(BaiduError::UnsupportedApi(self.config.api));
        }

        let token = self.token_manager.get_token()?;
        let url = self.build_url(&token);
        let image_data = Self::encode_image_form(image)?;

        let mut params = vec![("image", image_data)];

        // Add recognizeGranularity=char for location variants.
        if self.config.api.requests_boxes() {
            params.push(("recognizeGranularity", "char".to_owned()));
        }

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .map_err(BaiduError::Transport)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().unwrap_or_default();
            return Err(BaiduError::InvalidResponse(format!(
                "HTTP {status}: {body}"
            )));
        }

        let raw: serde_json::Value = resp.json().map_err(BaiduError::Transport)?;
        self.parser.parse(&raw)
    }

    /// Execute a Qianfan-OCR request (JSON with Bearer token).
    fn execute_qianfan(&self, image: &OcrImage) -> BaiduResult<OcrResult> {
        let png_bytes = encode_to_png(image)?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);

        // Qianfan-OCR uses the api_key as Bearer token directly.
        let url = self.build_qianfan_url();

        let body = serde_json::json!({
            "image": b64,
            "model": "Qianfan-OCR"
        });

        let resp = self
            .client
            .post(url)
            .header("Content-Type", "application/json; charset=utf-8")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&body)
            .send()
            .map_err(BaiduError::Transport)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().unwrap_or_default();
            return Err(BaiduError::InvalidResponse(format!(
                "HTTP {status}: {body}"
            )));
        }

        let raw: serde_json::Value = resp.json().map_err(BaiduError::Transport)?;
        self.parser.parse(&raw)
    }

    /// Get a reference to the engine configuration.
    #[must_use]
    pub fn config(&self) -> &BaiduConfig {
        &self.config
    }
}

impl OcrEngine for BaiduOcrEngine {
    fn recognize(
        &self,
        image: &OcrImage,
    ) -> std::result::Result<OcrResult, Box<dyn std::error::Error + Send + Sync>> {
        if self.config.api == BaiduApi::QianfanOcr {
            self.execute_qianfan(image).map_err(Into::into)
        } else {
            self.execute_standard(image).map_err(Into::into)
        }
    }

    fn name(&self) -> &'static str {
        self.config.api.engine_name()
    }

    fn languages(&self) -> &[&str] {
        &["zh", "en", "ja", "ko"]
    }

    fn level(&self) -> CapabilityLevel {
        CapabilityLevel::Cloud
    }
}

/// Encode the RGBA pixel data as PNG bytes.
///
/// # Errors
///
/// Returns [`BaiduError::ImageEncoding`] if the pixel data cannot be encoded.
fn encode_to_png(image: &OcrImage) -> BaiduResult<Vec<u8>> {
    let rgba_img = image::RgbaImage::from_raw(image.width, image.height, image.pixels.clone())
        .ok_or_else(|| {
            BaiduError::ImageEncoding(format!(
                "pixel buffer length {} does not match {}x{}x4",
                image.pixels.len(),
                image.width,
                image.height,
            ))
        })?;

    let dynamic = image::DynamicImage::ImageRgba8(rgba_img);
    let mut buf = Vec::new();
    dynamic
        .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|e| BaiduError::ImageEncoding(format!("PNG encoding failed: {e}")))?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(engine.level(), CapabilityLevel::Cloud);
    }

    #[test]
    fn test_encode_to_png() {
        let image = make_test_image(2, 2);
        let png = encode_to_png(&image).unwrap();
        assert!(png.starts_with(b"\x89PNG"));
    }

    #[test]
    fn test_encode_to_png_invalid() {
        let image = OcrImage::new(2, 2, vec![0u8; 10]); // wrong size
        let result = encode_to_png(&image);
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
        let image = make_test_image(1, 1);
        let encoded = BaiduOcrEngine::encode_image_form(&image).unwrap();
        // Should be URL-encoded base64.
        assert!(!encoded.is_empty());
        // Verify it decodes back to valid base64.
        let decoded = urlencoding::decode(&encoded).unwrap();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(decoded.as_ref())
            .unwrap();
        assert!(bytes.starts_with(b"\x89PNG"));
    }
}
