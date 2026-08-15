//! Configuration types for the Zhipu GLM-OCR engine.

use std::fmt;

/// Configuration for the Zhipu `BigModel` GLM-OCR engine.
///
/// # API Documentation
///
/// - Official guide: <https://docs.bigmodel.cn/cn/guide/models/vlm/glm-ocr>
/// - Model info: <https://huggingface.co/zai-org/GLM-OCR>
///
/// # Examples
///
/// ```
/// use easypdf_ocr::glm::GlmConfig;
///
/// let config = GlmConfig {
///     api_key: "your-api-key".to_owned(),
///     ..GlmConfig::default()
/// };
/// assert_eq!(config.model, "glm-ocr");
/// ```
#[derive(Clone)]
pub struct GlmConfig {
    /// API endpoint URL.
    ///
    /// Default: `"https://open.bigmodel.cn/api/paas/v4/layout_parsing"`
    /// (GLM-OCR layout parsing endpoint).
    pub endpoint: String,

    /// Zhipu `BigModel` API key.
    ///
    /// Required for authentication. Obtain from <https://open.bigmodel.cn/>.
    pub api_key: String,

    /// Model identifier.
    ///
    /// Default: `"glm-ocr"`.
    pub model: String,

    /// Optional language hint to force OCR in a specific language.
    ///
    /// Supported values include `"zh"` (Chinese), `"en"` (English), `"fr"` (French),
    /// `"es"` (Spanish), `"ru"` (Russian), `"de"` (German), `"ja"` (Japanese),
    /// `"ko"` (Korean), etc.
    ///
    /// When `None`, the engine auto-detects the language.
    pub language: Option<String>,

    /// Output format preference.
    ///
    /// Controls whether the response includes only text or text with
    /// positional bounding-box information.
    pub output_format: GlmOutputFormat,
}

impl Default for GlmConfig {
    fn default() -> Self {
        Self {
            endpoint: "https://open.bigmodel.cn/api/paas/v4/layout_parsing".to_owned(),
            model: "glm-ocr".to_owned(),
            api_key: String::new(),
            language: None,
            output_format: GlmOutputFormat::Text,
        }
    }
}

impl fmt::Debug for GlmConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GlmConfig")
            .field("endpoint", &self.endpoint)
            .field("api_key", &"***redacted***")
            .field("model", &self.model)
            .field("language", &self.language)
            .field("output_format", &self.output_format)
            .finish()
    }
}

/// Output format for GLM-OCR results.
///
/// Controls the level of detail returned by the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlmOutputFormat {
    /// Return only the extracted text content.
    Text,
    /// Return text along with positional bounding-box coordinates.
    TextWithBoxes,
}

impl fmt::Display for GlmOutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text => f.write_str("text"),
            Self::TextWithBoxes => f.write_str("text_with_boxes"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_endpoint() {
        let config = GlmConfig::default();
        assert_eq!(
            config.endpoint,
            "https://open.bigmodel.cn/api/paas/v4/layout_parsing"
        );
    }

    #[test]
    fn test_default_model() {
        let config = GlmConfig::default();
        assert_eq!(config.model, "glm-ocr");
    }

    #[test]
    fn test_default_language_is_none() {
        let config = GlmConfig::default();
        assert!(config.language.is_none());
    }

    #[test]
    fn test_default_output_format() {
        let config = GlmConfig::default();
        assert_eq!(config.output_format, GlmOutputFormat::Text);
    }

    #[test]
    fn test_output_format_display() {
        assert_eq!(GlmOutputFormat::Text.to_string(), "text");
        assert_eq!(
            GlmOutputFormat::TextWithBoxes.to_string(),
            "text_with_boxes"
        );
    }

    #[test]
    fn test_config_clone() {
        let config = GlmConfig {
            api_key: "key123".to_owned(),
            language: Some("zh".to_owned()),
            ..GlmConfig::default()
        };
        let cloned = config.clone();
        assert_eq!(cloned.api_key, "key123");
        assert_eq!(cloned.language, Some("zh".to_owned()));
    }

    #[test]
    fn test_api_key_redacted_in_debug() {
        let config = GlmConfig {
            api_key: "super-secret-key-12345".to_owned(),
            ..GlmConfig::default()
        };
        let debug = format!("{config:?}");
        assert!(
            !debug.contains("super-secret-key-12345"),
            "API key must not appear in Debug output"
        );
        assert!(
            debug.contains("redacted"),
            "Debug output should contain 'redacted'"
        );
        assert!(debug.contains("GlmConfig"));
    }
}
