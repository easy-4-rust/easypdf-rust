//! 智谱 GLM-OCR 引擎配置类型。

use std::fmt;

/// 智谱 `BigModel` GLM-OCR 引擎配置。
///
/// # API 文档
///
/// - 官方指南：<https://docs.bigmodel.cn/cn/guide/models/vlm/glm-ocr>
/// - 模型信息：<https://huggingface.co/zai-org/GLM-OCR>
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
    /// API 端点 URL。
    ///
    /// 默认值：`"https://open.bigmodel.cn/api/paas/v4/layout_parsing"`
    ///（GLM-OCR 版面解析端点）。
    pub endpoint: String,

    /// 智谱 `BigModel` API 密钥。
    ///
    /// 认证必需。从 <https://open.bigmodel.cn/> 获取。
    pub api_key: String,

    /// 模型标识符。
    ///
    /// 默认值：`"glm-ocr"`。
    pub model: String,

    /// 可选的语言提示，用于强制以特定语言进行 OCR。
    ///
    /// 支持的值包括 `"zh"`（中文）、`"en"`（英文）、`"fr"`（法文）、
    /// `"es"`（西班牙文）、`"ru"`（俄文）、`"de"`（德文）、`"ja"`（日文）、
    /// `"ko"`（韩文）等。
    ///
    /// 为 `None` 时，引擎自动检测语言。
    pub language: Option<String>,

    /// 输出格式偏好。
    ///
    /// 控制响应是仅包含文本还是同时包含位置边界框信息。
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

/// GLM-OCR 结果的输出格式。
///
/// 控制引擎返回的详细程度。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlmOutputFormat {
    /// 仅返回提取的文本内容。
    Text,
    /// 返回文本及位置边界框坐标。
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
