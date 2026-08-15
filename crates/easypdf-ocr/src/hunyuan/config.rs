//! 腾讯云 OCR 引擎配置。
//!
//! 提供 [`HunyuanConfig`] 用于配置 OCR 客户端，[`HunyuanMode`]
//! 用于选择识别模式。
//!
//! # 入门指南
//!
//! 1. 注册[腾讯云](https://cloud.tencent.com/)账号。
//! 2. 前往 [OCR 控制台](https://console.cloud.tencent.com/ocr/overview)
//!    开通 OCR 服务。
//! 3. 从[密钥管理](https://console.cloud.tencent.com/cam/capi)页面
//!    获取 `SecretId` 和 `SecretKey`。
//! 4. 使用凭据和所需模式创建 [`HunyuanConfig`]。
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

/// 腾讯云 OCR 引擎配置。
///
/// 持有 OCR API 的凭据、端点和模式。
///
/// # 安全
///
/// [`Debug`] 实现会脱敏 `secret_id` 和 `secret_key`，
/// 防止凭据意外泄漏到日志中。
#[derive(Clone)]
pub struct HunyuanConfig {
    /// 腾讯云 Secret ID。
    pub secret_id: String,
    /// 腾讯云 Secret Key。
    pub secret_key: String,
    /// 地域（如 `"ap-guangzhou"`）。
    pub region: String,
    /// 服务名称（默认：`"ocr"`）。
    pub service: String,
    /// API 版本（默认：`"2018-11-19"`）。
    pub version: String,
    /// 完整端点 URL（默认：`"https://ocr.tencentcloudapi.com"`）。
    pub endpoint: String,
    /// OCR 识别模式。
    pub mode: HunyuanMode,
    /// 可选的语言提示（如 `"zh"`、`"en"`、`"ja"`、`"ko"`）。
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

/// 腾讯云 OCR 识别模式。
///
/// 选择调用哪个 API 操作：
///
/// | 模式 | API 操作 | 描述 |
/// |------|----------|------|
/// | [`GeneralBasic`](HunyuanMode::GeneralBasic) | `GeneralBasicOCR` | 通用文字识别（基础版） |
/// | [`SmartStructural`](HunyuanMode::SmartStructural) | `SmartStructuralOCR` | 文档提取（基础版） |
/// | [`GeneralAccurate`](HunyuanMode::GeneralAccurate) | `GeneralAccurateOCR` | 通用文字识别（高精度版，含位置） |
///
/// # API 参考
///
/// - [GeneralBasicOCR](https://cloud.tencent.com/document/product/866/36210)
/// - [SmartStructuralOCR](https://cloud.tencent.com/document/product/866/119452)
/// - [GeneralAccurateOCR](https://cloud.tencent.com/document/product/866/34936)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HunyuanMode {
    /// 通用文字识别（基础版）。
    ///
    /// 使用 `GeneralBasicOCR` API 操作。适用于大多数常见文字识别场景，
    /// 具有良好的精度和低延迟。
    GeneralBasic,

    /// 文档提取（基础版）。
    ///
    /// 使用 `SmartStructuralOCR` API 操作（别名 `ExtractDocBasic`）。
    /// 从文档、收据、发票等提取结构化文本。返回结构化字段和全文。
    SmartStructural,

    /// 通用文字识别（高精度版）。
    ///
    /// 使用 `GeneralAccurateOCR` API 操作。精度高于基础版，
    /// 包含词级边界框坐标。
    GeneralAccurate,
}

impl HunyuanMode {
    /// 返回此模式的腾讯云 API 操作名称。
    #[must_use]
    pub fn action_name(&self) -> &'static str {
        match self {
            Self::GeneralBasic => "GeneralBasicOCR",
            Self::SmartStructural => "SmartStructuralOCR",
            Self::GeneralAccurate => "GeneralAccurateOCR",
        }
    }

    /// 返回此模式的引擎名称前缀。
    #[must_use]
    pub fn engine_name(&self) -> &'static str {
        match self {
            Self::GeneralBasic => "hunyuan-general-basic",
            Self::SmartStructural => "hunyuan-smart-structural",
            Self::GeneralAccurate => "hunyuan-general-accurate",
        }
    }

    /// 若此模式使用 `TextDetections` 响应格式则返回 `true`。
    ///
    /// 通用 OCR 模式（`GeneralBasic`、`GeneralAccurate`）返回
    /// `TextDetections[].DetectedText`。文档提取模式
    /// （`SmartStructural`）返回 `WordList[].Text`。
    #[must_use]
    pub fn uses_text_detections(&self) -> bool {
        matches!(self, Self::GeneralBasic | Self::GeneralAccurate)
    }
}

/// 脱敏字符串，仅显示前 4 个和后 4 个字符。
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
