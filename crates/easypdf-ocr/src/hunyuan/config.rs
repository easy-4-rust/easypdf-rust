//! Configuration for the Tencent Cloud OCR engine.
//!
//! Provides [`HunyuanConfig`] for configuring the OCR client and [`HunyuanMode`]
//! for selecting the recognition mode.
//!
//! # Getting Started
//!
//! 1. Sign up for a [Tencent Cloud](https://cloud.tencent.com/) account.
//! 2. Go to the [OCR console](https://console.cloud.tencent.com/ocr/overview)
//!    and activate the OCR service.
//! 3. Obtain your `SecretId` and `SecretKey` from the
//!    [API Key Management](https://console.cloud.tencent.com/cam/capi) page.
//! 4. Create a [`HunyuanConfig`] with your credentials and desired mode.
//!
//! # Example
//!
//! ```rust
//! use easypdf_ocr::hunyuan::{HunyuanConfig, HunyuanMode};
//!
//! let config = HunyuanConfig {
//!     secret_id: "your-secret-id".to_string(),
//!     secret_key: "your-secret-key".to_string(),
//!     mode: HunyuanMode::GeneralBasic,
//!     ..HunyuanConfig::default()
//! };
//! ```

use std::fmt;

/// Tencent Cloud OCR engine configuration.
///
/// Holds credentials, endpoint, and mode for the OCR API.
///
/// # Security
///
/// The [`Debug`] implementation redacts `secret_id` and `secret_key` to
/// prevent accidental credential leakage in logs.
#[derive(Clone)]
pub struct HunyuanConfig {
    /// Tencent Cloud secret ID.
    pub secret_id: String,
    /// Tencent Cloud secret key.
    pub secret_key: String,
    /// Region (e.g., `"ap-guangzhou"`).
    pub region: String,
    /// Service name (default: `"ocr"`).
    pub service: String,
    /// API version (default: `"2018-11-19"`).
    pub version: String,
    /// Full endpoint URL (default: `"https://ocr.tencentcloudapi.com"`).
    pub endpoint: String,
    /// OCR recognition mode.
    pub mode: HunyuanMode,
    /// Optional language hint (e.g., `"zh"`, `"en"`, `"ja"`, `"ko"`).
    pub language: Option<String>,
}

impl fmt::Debug for HunyuanConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HunyuanConfig")
            .field("secret_id", &redact(&self.secret_id))
            .field("secret_key", &"***")
            .field("region", &self.region)
            .field("service", &self.service)
            .field("version", &self.version)
            .field("endpoint", &self.endpoint)
            .field("mode", &self.mode)
            .field("language", &self.language)
            .finish()
    }
}

impl Default for HunyuanConfig {
    fn default() -> Self {
        Self {
            secret_id: String::new(),
            secret_key: String::new(),
            region: "ap-guangzhou".to_string(),
            service: "ocr".to_string(),
            version: "2018-11-19".to_string(),
            endpoint: "https://ocr.tencentcloudapi.com".to_string(),
            mode: HunyuanMode::GeneralBasic,
            language: None,
        }
    }
}

/// OCR recognition mode for Tencent Cloud OCR.
///
/// Selects which API action to call:
///
/// | Mode | API Action | Description |
/// |------|------------|-------------|
/// | [`GeneralBasic`](HunyuanMode::GeneralBasic) | `GeneralBasicOCR` | General text recognition (basic) |
/// | [`SmartStructural`](HunyuanMode::SmartStructural) | `SmartStructuralOCR` | Document extraction (basic version) |
/// | [`GeneralAccurate`](HunyuanMode::GeneralAccurate) | `GeneralAccurateOCR` | General text recognition (accurate, with position) |
///
/// # API References
///
/// - [GeneralBasicOCR](https://cloud.tencent.com/document/product/866/36210)
/// - [SmartStructuralOCR](https://cloud.tencent.com/document/product/866/119452)
/// - [GeneralAccurateOCR](https://cloud.tencent.com/document/product/866/34936)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HunyuanMode {
    /// General text recognition (basic version).
    ///
    /// Uses the `GeneralBasicOCR` API action. Suitable for most common
    /// text recognition scenarios with good accuracy and low latency.
    GeneralBasic,

    /// Document extraction (basic version).
    ///
    /// Uses the `SmartStructuralOCR` API action (alias `ExtractDocBasic`).
    /// Extracts structured text from documents, receipts, invoices, etc.
    /// Returns both structured fields and full text.
    SmartStructural,

    /// General text recognition (accurate version).
    ///
    /// Uses the `GeneralAccurateOCR` API action. Higher accuracy than basic,
    /// includes word-level bounding box coordinates.
    GeneralAccurate,
}

impl HunyuanMode {
    /// Returns the Tencent Cloud API action name for this mode.
    #[must_use]
    pub fn action_name(&self) -> &'static str {
        match self {
            Self::GeneralBasic => "GeneralBasicOCR",
            Self::SmartStructural => "SmartStructuralOCR",
            Self::GeneralAccurate => "GeneralAccurateOCR",
        }
    }

    /// Returns the engine name prefix for this mode.
    #[must_use]
    pub fn engine_name(&self) -> &'static str {
        match self {
            Self::GeneralBasic => "hunyuan-general-basic",
            Self::SmartStructural => "hunyuan-smart-structural",
            Self::GeneralAccurate => "hunyuan-general-accurate",
        }
    }

    /// Returns `true` if this mode uses `TextDetections` response format.
    ///
    /// General OCR modes (`GeneralBasic`, `GeneralAccurate`) return
    /// `TextDetections[].DetectedText`. Document extraction modes
    /// (`SmartStructural`) return `WordList[].Text`.
    #[must_use]
    pub fn uses_text_detections(&self) -> bool {
        matches!(self, Self::GeneralBasic | Self::GeneralAccurate)
    }
}

/// Redact a string, showing only the first 4 and last 4 characters.
fn redact(s: &str) -> String {
    if s.len() <= 8 {
        "***".to_owned()
    } else {
        format!("{}...{}", &s[..4], &s[s.len() - 4..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = HunyuanConfig::default();
        assert!(config.secret_id.is_empty());
        assert!(config.secret_key.is_empty());
        assert_eq!(config.region, "ap-guangzhou");
        assert_eq!(config.service, "ocr");
        assert_eq!(config.version, "2018-11-19");
        assert_eq!(config.endpoint, "https://ocr.tencentcloudapi.com");
        assert_eq!(config.mode, HunyuanMode::GeneralBasic);
        assert!(config.language.is_none());
    }

    #[test]
    fn test_debug_redacts_secrets() {
        let config = HunyuanConfig {
            secret_id: "AKID1234567890".to_string(),
            secret_key: "super-secret-key-value".to_string(),
            ..HunyuanConfig::default()
        };
        let debug = format!("{config:?}");
        assert!(!debug.contains("super-secret-key-value"));
        assert!(!debug.contains("AKID1234567890"));
        assert!(debug.contains("***"));
        assert!(debug.contains("ocr"));
    }

    #[test]
    fn test_mode_action_names() {
        assert_eq!(HunyuanMode::GeneralBasic.action_name(), "GeneralBasicOCR");
        assert_eq!(
            HunyuanMode::SmartStructural.action_name(),
            "SmartStructuralOCR"
        );
        assert_eq!(
            HunyuanMode::GeneralAccurate.action_name(),
            "GeneralAccurateOCR"
        );
    }

    #[test]
    fn test_mode_engine_names() {
        assert_eq!(
            HunyuanMode::GeneralBasic.engine_name(),
            "hunyuan-general-basic"
        );
        assert_eq!(
            HunyuanMode::SmartStructural.engine_name(),
            "hunyuan-smart-structural"
        );
        assert_eq!(
            HunyuanMode::GeneralAccurate.engine_name(),
            "hunyuan-general-accurate"
        );
    }

    #[test]
    fn test_mode_uses_text_detections() {
        assert!(HunyuanMode::GeneralBasic.uses_text_detections());
        assert!(HunyuanMode::GeneralAccurate.uses_text_detections());
        assert!(!HunyuanMode::SmartStructural.uses_text_detections());
    }

    #[test]
    fn test_mode_equality() {
        assert_eq!(HunyuanMode::GeneralBasic, HunyuanMode::GeneralBasic);
        assert_ne!(HunyuanMode::GeneralBasic, HunyuanMode::SmartStructural);
    }

    #[test]
    fn test_mode_copy() {
        let mode = HunyuanMode::GeneralBasic;
        let mode_copy = mode;
        assert_eq!(mode, mode_copy);
    }
}
