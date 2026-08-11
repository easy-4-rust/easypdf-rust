//! Configuration for Baidu Cloud OCR engines.
//!
//! Baidu Cloud offers 15 OCR API endpoints plus Qianfan-OCR (a large model
//! endpoint). Each endpoint is represented by a [`BaiduApi`] variant.
//!
//! # Authentication
//!
//! Baidu OCR uses OAuth 2.0 client-credentials flow:
//! 1. Exchange `api_key` + `secret_key` for an `access_token` via the token URL.
//! 2. Pass `access_token` as a URL query parameter on OCR requests.
//!
//! Tokens are valid for ~30 days; the engine caches them automatically.
//!
//! # API Endpoints
//!
//! All standard endpoints are at:
//! `https://aip.baidubce.com/rest/2.0/ocr/v1/{action}`
//!
//! | API | Action Path | Description |
//! |-----|-------------|-------------|
//! | [`BaiduApi::GeneralBasic`] | `general_basic` | General text recognition (standard) |
//! | [`BaiduApi::GeneralAccurate`] | `accurate_basic` | General text recognition (high accuracy) |
//! | [`BaiduApi::GeneralBasicWithLocation`] | `general_basic` | Standard with bounding boxes |
//! | [`BaiduApi::GeneralAccurateWithLocation`] | `accurate_basic` | High accuracy with bounding boxes |
//! | [`BaiduApi::TableRecognitionV2`] | `table` | Table structure recognition |
//! | [`BaiduApi::WebImage`] | `webimage` | Web image text recognition |
//! | [`BaiduApi::WebImageWithLocation`] | `webimage_loc` | Web image with bounding boxes |
//! | [`BaiduApi::OfficeDocument`] | `doc_analysis_office` | Office document recognition |
//! | [`BaiduApi::Handwriting`] | `handwriting` | Handwriting recognition |
//! | [`BaiduApi::Seal`] | `seal` | Seal/stamp recognition |
//! | [`BaiduApi::Digit`] | `numbers` | Digit recognition |
//! | [`BaiduApi::Qrcode`] | `qrcode` | QR code recognition |
//! | [`BaiduApi::Structured`] | `smart_struct` | Intelligent structuring |
//! | [`BaiduApi::DocParser`] | `doc_parser` | Document parsing (basic) |
//! | [`BaiduApi::DocParserPaddle`] | `doc_parser_paddle` | Document parsing (PaddleOCR-VL) |
//! | [`BaiduApi::QianfanOcr`] | N/A | Qianfan large model OCR |
//!
//! # Getting Credentials
//!
//! 1. Create an application at [Baidu AI Cloud Console](https://console.bce.baidu.com/ai/#/ai/ocr/overview/index).
//! 2. Obtain the API Key and Secret Key from the application credentials page.

use std::fmt;

/// Baidu Cloud OCR API endpoint selector.
///
/// Each variant corresponds to a specific OCR capability offered by Baidu Cloud.
/// Use [`BaiduApi::path()`](BaiduApi::path) to get the URL path segment and
/// [`BaiduApi::engine_name()`](BaiduApi::engine_name) for a human-readable label.
///
/// # Examples
///
/// ```
/// use easypdf_ocr::baidu::BaiduApi;
///
/// let api = BaiduApi::GeneralAccurate;
/// assert_eq!(api.path(), "accurate_basic");
/// assert_eq!(api.engine_name(), "baidu-general-accurate");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BaiduApi {
    /// General text recognition (standard version).
    /// Endpoint: `general_basic`.
    GeneralBasic,
    /// General text recognition (high-accuracy version).
    /// Endpoint: `accurate_basic`.
    GeneralAccurate,
    /// General text recognition (standard with bounding boxes).
    /// Same endpoint as `GeneralBasic` but with `recognizeGranularity=char`.
    GeneralBasicWithLocation,
    /// General text recognition (high-accuracy with bounding boxes).
    /// Same endpoint as `GeneralAccurate` but with `recognizeGranularity=char`.
    GeneralAccurateWithLocation,
    /// Table structure recognition V2.
    /// Endpoint: `table`.
    TableRecognitionV2,
    /// Web image text recognition.
    /// Endpoint: `webimage`.
    WebImage,
    /// Web image text recognition with bounding boxes.
    /// Endpoint: `webimage_loc`.
    WebImageWithLocation,
    /// Office document recognition.
    /// Endpoint: `doc_analysis_office`.
    OfficeDocument,
    /// Handwriting recognition.
    /// Endpoint: `handwriting`.
    Handwriting,
    /// Seal/stamp recognition.
    /// Endpoint: `seal`.
    Seal,
    /// Digit recognition.
    /// Endpoint: `numbers`.
    Digit,
    /// QR code recognition.
    /// Endpoint: `qrcode`.
    Qrcode,
    /// Intelligent structuring recognition.
    /// Endpoint: `smart_struct`.
    Structured,
    /// Document parsing (basic version).
    /// Endpoint: `doc_parser`.
    DocParser,
    /// Document parsing (PaddleOCR-VL).
    /// Endpoint: `doc_parser_paddle`.
    DocParserPaddle,
    /// Qianfan-OCR large model (uses a different API endpoint).
    QianfanOcr,
}

impl BaiduApi {
    /// URL path segment for this API endpoint.
    ///
    /// For standard APIs, the full URL is `{base_endpoint}/{path()}`.
    /// For [`QianfanOcr`](BaiduApi::QianfanOcr), the path is used differently
    /// (see [`BaiduConfig::qianfan_endpoint`]).
    #[must_use]
    pub const fn path(&self) -> &'static str {
        match self {
            Self::GeneralBasic | Self::GeneralBasicWithLocation => "general_basic",
            Self::GeneralAccurate | Self::GeneralAccurateWithLocation => "accurate_basic",
            Self::TableRecognitionV2 => "table",
            Self::WebImage => "webimage",
            Self::WebImageWithLocation => "webimage_loc",
            Self::OfficeDocument => "doc_analysis_office",
            Self::Handwriting => "handwriting",
            Self::Seal => "seal",
            Self::Digit => "numbers",
            Self::Qrcode => "qrcode",
            Self::Structured => "smart_struct",
            Self::DocParser => "doc_parser",
            Self::DocParserPaddle => "doc_parser_paddle",
            Self::QianfanOcr => "qianfan_ocr",
        }
    }

    /// Whether this variant requests character-level bounding boxes.
    ///
    /// When `true`, the request includes `recognizeGranularity=char` to obtain
    /// per-character location data.
    #[must_use]
    pub const fn requests_boxes(&self) -> bool {
        matches!(
            self,
            Self::GeneralBasicWithLocation
                | Self::GeneralAccurateWithLocation
                | Self::WebImageWithLocation
        )
    }

    /// Whether the response typically contains text (`words_result`).
    ///
    /// Most APIs return `words_result`; table and doc APIs return structured data.
    #[must_use]
    pub const fn returns_text(&self) -> bool {
        !matches!(
            self,
            Self::TableRecognitionV2
                | Self::OfficeDocument
                | Self::Seal
                | Self::Qrcode
                | Self::Structured
                | Self::DocParser
                | Self::DocParserPaddle
        )
    }

    /// Whether the response contains bounding-box coordinates.
    #[must_use]
    pub const fn returns_boxes(&self) -> bool {
        matches!(
            self,
            Self::GeneralBasicWithLocation
                | Self::GeneralAccurateWithLocation
                | Self::WebImageWithLocation
                | Self::TableRecognitionV2
                | Self::DocParser
                | Self::DocParserPaddle
        )
    }

    /// Human-readable engine name for logging and identification.
    #[must_use]
    pub const fn engine_name(&self) -> &'static str {
        match self {
            Self::GeneralBasic => "baidu-general-basic",
            Self::GeneralAccurate => "baidu-general-accurate",
            Self::GeneralBasicWithLocation => "baidu-general-basic-loc",
            Self::GeneralAccurateWithLocation => "baidu-general-accurate-loc",
            Self::TableRecognitionV2 => "baidu-table-v2",
            Self::WebImage => "baidu-webimage",
            Self::WebImageWithLocation => "baidu-webimage-loc",
            Self::OfficeDocument => "baidu-office-doc",
            Self::Handwriting => "baidu-handwriting",
            Self::Seal => "baidu-seal",
            Self::Digit => "baidu-digit",
            Self::Qrcode => "baidu-qrcode",
            Self::Structured => "baidu-structured",
            Self::DocParser => "baidu-doc-parser",
            Self::DocParserPaddle => "baidu-doc-parser-paddle",
            Self::QianfanOcr => "baidu-qianfan-ocr",
        }
    }

    /// Whether this API is currently supported by the engine.
    ///
    /// Unsupported variants will return [`BaiduError::UnsupportedApi`] at runtime.
    #[must_use]
    pub const fn is_supported(&self) -> bool {
        !matches!(self, Self::DocParser | Self::DocParserPaddle)
    }
}

impl fmt::Display for BaiduApi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.engine_name())
    }
}

/// Configuration for a Baidu Cloud OCR engine.
///
/// Holds credentials, API selection, and endpoint URLs. The [`Default`] impl
/// uses the standard Baidu Cloud OCR base endpoint with [`BaiduApi::GeneralBasic`].
///
/// # Examples
///
/// ```
/// use easypdf_ocr::baidu::{BaiduConfig, BaiduApi};
///
/// let config = BaiduConfig {
///     api_key: "your-api-key".to_owned(),
///     secret_key: "your-secret-key".to_owned(),
///     api: BaiduApi::GeneralAccurate,
///     ..BaiduConfig::default()
/// };
/// ```
#[derive(Clone)]
pub struct BaiduConfig {
    /// Baidu Cloud API key (client ID).
    pub api_key: String,
    /// Baidu Cloud secret key (client secret).
    pub secret_key: String,
    /// Which OCR API endpoint to use.
    pub api: BaiduApi,
    /// Base URL for standard OCR endpoints.
    ///
    /// Default: `https://aip.baidubce.com/rest/2.0/ocr/v1`.
    /// The API path is appended automatically.
    pub endpoint: String,
    /// OAuth token exchange URL.
    ///
    /// Default: `https://aip.baidubce.com/oauth/2.0/token`.
    pub token_url: String,
    /// Qianfan-OCR endpoint (used only when `api` is [`BaiduApi::QianfanOcr`]).
    ///
    /// Default: `https://qianfan.baidubce.com/v2/app/tool`.
    pub qianfan_endpoint: String,
}

impl Default for BaiduConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            secret_key: String::new(),
            api: BaiduApi::GeneralBasic,
            endpoint: "https://aip.baidubce.com/rest/2.0/ocr/v1".to_owned(),
            token_url: "https://aip.baidubce.com/oauth/2.0/token".to_owned(),
            qianfan_endpoint: "https://qianfan.baidubce.com/v2/app/tool".to_owned(),
        }
    }
}

impl fmt::Debug for BaiduConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BaiduConfig")
            .field("api_key", &"***redacted***")
            .field("secret_key", &"***redacted***")
            .field("api", &self.api)
            .field("endpoint", &self.endpoint)
            .field("token_url", &self.token_url)
            .field("qianfan_endpoint", &self.qianfan_endpoint)
            .finish()
    }
}

/// Error type for Baidu OCR operations.
#[derive(Debug, thiserror::Error)]
pub enum BaiduError {
    /// HTTP transport error (connection, timeout, DNS).
    #[error("HTTP transport error: {0}")]
    Transport(#[from] reqwest::Error),

    /// OAuth token exchange failed.
    #[error("OAuth token exchange failed: {0}")]
    Auth(String),

    /// The Baidu API returned an application-level error.
    #[error("Baidu API error {code}: {message}")]
    Api {
        /// Baidu error code.
        code: i64,
        /// Baidu error message.
        message: String,
    },

    /// The response JSON could not be parsed.
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// The selected API variant is not yet implemented.
    #[error("Unsupported Baidu API: {0}")]
    UnsupportedApi(BaiduApi),

    /// Image encoding failed.
    #[error("Image encoding failed: {0}")]
    ImageEncoding(String),
}

/// Convenience result type for Baidu OCR operations.
pub type BaiduResult<T> = std::result::Result<T, BaiduError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_baidu_api_path() {
        assert_eq!(BaiduApi::GeneralBasic.path(), "general_basic");
        assert_eq!(BaiduApi::GeneralAccurate.path(), "accurate_basic");
        assert_eq!(
            BaiduApi::GeneralBasicWithLocation.path(),
            "general_basic"
        );
        assert_eq!(
            BaiduApi::GeneralAccurateWithLocation.path(),
            "accurate_basic"
        );
        assert_eq!(BaiduApi::TableRecognitionV2.path(), "table");
        assert_eq!(BaiduApi::WebImage.path(), "webimage");
        assert_eq!(BaiduApi::WebImageWithLocation.path(), "webimage_loc");
        assert_eq!(BaiduApi::OfficeDocument.path(), "doc_analysis_office");
        assert_eq!(BaiduApi::Handwriting.path(), "handwriting");
        assert_eq!(BaiduApi::Seal.path(), "seal");
        assert_eq!(BaiduApi::Digit.path(), "numbers");
        assert_eq!(BaiduApi::Qrcode.path(), "qrcode");
        assert_eq!(BaiduApi::Structured.path(), "smart_struct");
        assert_eq!(BaiduApi::DocParser.path(), "doc_parser");
        assert_eq!(BaiduApi::DocParserPaddle.path(), "doc_parser_paddle");
        assert_eq!(BaiduApi::QianfanOcr.path(), "qianfan_ocr");
    }

    #[test]
    fn test_baidu_api_engine_name() {
        assert_eq!(BaiduApi::GeneralBasic.engine_name(), "baidu-general-basic");
        assert_eq!(
            BaiduApi::GeneralAccurate.engine_name(),
            "baidu-general-accurate"
        );
        assert_eq!(
            BaiduApi::TableRecognitionV2.engine_name(),
            "baidu-table-v2"
        );
        assert_eq!(BaiduApi::QianfanOcr.engine_name(), "baidu-qianfan-ocr");
    }

    #[test]
    fn test_baidu_api_requests_boxes() {
        assert!(!BaiduApi::GeneralBasic.requests_boxes());
        assert!(BaiduApi::GeneralBasicWithLocation.requests_boxes());
        assert!(BaiduApi::GeneralAccurateWithLocation.requests_boxes());
        assert!(!BaiduApi::TableRecognitionV2.requests_boxes());
        assert!(BaiduApi::WebImageWithLocation.requests_boxes());
    }

    #[test]
    fn test_baidu_api_is_supported() {
        assert!(BaiduApi::GeneralBasic.is_supported());
        assert!(BaiduApi::GeneralAccurate.is_supported());
        assert!(BaiduApi::TableRecognitionV2.is_supported());
        assert!(BaiduApi::QianfanOcr.is_supported());
        // Newly supported:
        assert!(BaiduApi::OfficeDocument.is_supported());
        assert!(BaiduApi::Handwriting.is_supported());
        assert!(BaiduApi::Seal.is_supported());
        assert!(BaiduApi::Digit.is_supported());
        assert!(BaiduApi::Qrcode.is_supported());
        assert!(BaiduApi::Structured.is_supported());
        // Still unsupported (async APIs):
        assert!(!BaiduApi::DocParser.is_supported());
        assert!(!BaiduApi::DocParserPaddle.is_supported());
    }

    #[test]
    fn test_baidu_config_default() {
        let config = BaiduConfig::default();
        assert_eq!(config.api, BaiduApi::GeneralBasic);
        assert!(config.endpoint.contains("baidubce.com"));
        assert!(config.token_url.contains("baidubce.com"));
        assert!(config.api_key.is_empty());
    }

    #[test]
    fn test_baidu_api_display() {
        assert_eq!(format!("{}", BaiduApi::GeneralBasic), "baidu-general-basic");
        assert_eq!(
            format!("{}", BaiduApi::TableRecognitionV2),
            "baidu-table-v2"
        );
    }

    #[test]
    fn test_baidu_api_returns_text() {
        assert!(BaiduApi::GeneralBasic.returns_text());
        assert!(BaiduApi::Handwriting.returns_text());
        assert!(BaiduApi::Digit.returns_text());
        assert!(!BaiduApi::TableRecognitionV2.returns_text());
        assert!(!BaiduApi::OfficeDocument.returns_text());
        assert!(!BaiduApi::Seal.returns_text());
        assert!(!BaiduApi::Qrcode.returns_text());
        assert!(!BaiduApi::Structured.returns_text());
    }

    #[test]
    fn test_debug_redacts_both_keys() {
        let config = BaiduConfig {
            api_key: "AK-abcdefghijklmnop".to_owned(),
            secret_key: "SK-extremely-secret-value".to_owned(),
            ..BaiduConfig::default()
        };
        let debug = format!("{config:?}");
        assert!(!debug.contains("AK-abcdefghijklmnop"), "api_key must not appear in Debug");
        assert!(!debug.contains("SK-extremely-secret-value"), "secret_key must not appear in Debug");
        assert!(debug.contains("redacted"), "Debug output should contain 'redacted'");
        assert!(debug.contains("BaiduConfig"));
    }
}
