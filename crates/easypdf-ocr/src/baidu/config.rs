//! 百度云 OCR 引擎配置。
//!
//! 百度云提供 15 个 OCR API 端点加千帆 OCR（大模型端点）。
//! 每个端点由 [`BaiduApi`] 变体表示。
//!
//! # 认证
//!
//! 百度 OCR 使用 OAuth 2.0 客户端凭证流程：
//! 1. 通过令牌 URL 将 `api_key` + `secret_key` 交换为 `access_token`。
//! 2. 在 OCR 请求中将 `access_token` 作为 URL 查询参数传递。
//!
//! 令牌有效期约 30 天；引擎自动缓存。
//!
//! # API 端点
//!
//! 所有标准端点位于：
//! `https://aip.baidubce.com/rest/2.0/ocr/v1/{action}`
//!
//! | API | 操作路径 | 描述 |
//! |-----|----------|------|
//! | [`BaiduApi::GeneralBasic`] | `general_basic` | 通用文字识别（标准版） |
//! | [`BaiduApi::GeneralAccurate`] | `accurate_basic` | 通用文字识别（高精度版） |
//! | [`BaiduApi::GeneralBasicWithLocation`] | `general_basic` | 标准版含边界框 |
//! | [`BaiduApi::GeneralAccurateWithLocation`] | `accurate_basic` | 高精度版含边界框 |
//! | [`BaiduApi::TableRecognitionV2`] | `table` | 表格结构识别 |
//! | [`BaiduApi::WebImage`] | `webimage` | 网络图片文字识别 |
//! | [`BaiduApi::WebImageWithLocation`] | `webimage_loc` | 网络图片含边界框 |
//! | [`BaiduApi::OfficeDocument`] | `doc_analysis_office` | 办公文档识别 |
//! | [`BaiduApi::Handwriting`] | `handwriting` | 手写体识别 |
//! | [`BaiduApi::Seal`] | `seal` | 印章识别 |
//! | [`BaiduApi::Digit`] | `numbers` | 数字识别 |
//! | [`BaiduApi::Qrcode`] | `qrcode` | 二维码识别 |
//! | [`BaiduApi::Structured`] | `smart_struct` | 智能结构化 |
//! | [`BaiduApi::DocParser`] | `doc_parser` | 文档解析（基础版） |
//! | [`BaiduApi::DocParserPaddle`] | `doc_parser_paddle` | 文档解析（PaddleOCR-VL） |
//! | [`BaiduApi::QianfanOcr`] | N/A | 千帆大模型 OCR |
//!
//! # 获取凭据
//!
//! 1. 在[百度智能云控制台](https://console.bce.baidu.com/ai/#/ai/ocr/overview/index)创建应用。
//! 2. 从应用凭据页面获取 API Key 和 Secret Key。

use std::fmt;

/// 百度云 OCR API 端点选择器。
///
/// 每个变体对应百度云提供的一种特定 OCR 能力。
/// 使用 [`BaiduApi::path()`](BaiduApi::path) 获取 URL 路径段，
/// 使用 [`BaiduApi::engine_name()`](BaiduApi::engine_name) 获取人类可读标签。
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
    /// 通用文字识别（标准版）。
    /// 端点：`general_basic`。
    GeneralBasic,
    /// 通用文字识别（高精度版）。
    /// 端点：`accurate_basic`。
    GeneralAccurate,
    /// 通用文字识别（标准版含边界框）。
    /// 与 `GeneralBasic` 相同端点，但带 `recognizeGranularity=char`。
    GeneralBasicWithLocation,
    /// 通用文字识别（高精度版含边界框）。
    /// 与 `GeneralAccurate` 相同端点，但带 `recognizeGranularity=char`。
    GeneralAccurateWithLocation,
    /// 表格结构识别 V2。
    /// 端点：`table`。
    TableRecognitionV2,
    /// 网络图片文字识别。
    /// 端点：`webimage`。
    WebImage,
    /// 网络图片文字识别含边界框。
    /// 端点：`webimage_loc`。
    WebImageWithLocation,
    /// 办公文档识别。
    /// 端点：`doc_analysis_office`。
    OfficeDocument,
    /// 手写体识别。
    /// 端点：`handwriting`。
    Handwriting,
    /// 印章识别。
    /// 端点：`seal`。
    Seal,
    /// 数字识别。
    /// 端点：`numbers`。
    Digit,
    /// 二维码识别。
    /// 端点：`qrcode`。
    Qrcode,
    /// 智能结构化识别。
    /// 端点：`smart_struct`。
    Structured,
    /// 文档解析（基础版）。
    /// 端点：`doc_parser`。
    DocParser,
    /// 文档解析（PaddleOCR-VL）。
    /// 端点：`doc_parser_paddle`。
    DocParserPaddle,
    /// 千帆 OCR 大模型（使用不同的 API 端点）。
    QianfanOcr,
}

impl BaiduApi {
    /// 此 API 端点的 URL 路径段。
    ///
    /// 对于标准 API，完整 URL 为 `{base_endpoint}/{path()}`。
    /// 对于 [`QianfanOcr`](BaiduApi::QianfanOcr)，路径的使用方式不同
    ///（参见 [`BaiduConfig::qianfan_endpoint`]）。
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

    /// 此变体是否请求字符级边界框。
    ///
    /// 为 `true` 时，请求包含 `recognizeGranularity=char` 以获取逐字符位置数据。
    #[must_use]
    pub const fn requests_boxes(&self) -> bool {
        matches!(
            self,
            Self::GeneralBasicWithLocation
                | Self::GeneralAccurateWithLocation
                | Self::WebImageWithLocation
        )
    }

    /// 响应是否通常包含文本（`words_result`）。
    ///
    /// 大多数 API 返回 `words_result`；表格和文档 API 返回结构化数据。
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

    /// 响应是否包含边界框坐标。
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

    /// 用于日志和标识的人类可读引擎名称。
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

    /// 此 API 是否当前受引擎支持。
    ///
    /// 不支持的变体在运行时将返回 [`BaiduError::UnsupportedApi`]。
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

/// 百度云 OCR 引擎配置。
///
/// 持有凭据、API 选择和端点 URL。[`Default`] 实现使用
/// 标准百度云 OCR 基础端点和 [`BaiduApi::GeneralBasic`]。
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
    /// 百度云 API 密钥（客户端 ID）。
    pub api_key: String,
    /// 百度云密钥（客户端密钥）。
    pub secret_key: String,
    /// 使用哪个 OCR API 端点。
    pub api: BaiduApi,
    /// 标准 OCR 端点的基础 URL。
    ///
    /// 默认值：`https://aip.baidubce.com/rest/2.0/ocr/v1`。
    /// API 路径会自动追加。
    pub endpoint: String,
    /// OAuth 令牌交换 URL。
    ///
    /// 默认值：`https://aip.baidubce.com/oauth/2.0/token`。
    pub token_url: String,
    /// 千帆 OCR 端点（仅当 `api` 为 [`BaiduApi::QianfanOcr`] 时使用）。
    ///
    /// 默认值：`https://qianfan.baidubce.com/v2/app/tool`。
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

/// 百度 OCR 操作的错误类型。
#[derive(Debug, thiserror::Error)]
pub enum BaiduError {
    /// HTTP 传输错误（连接、超时、DNS）。
    #[error("HTTP transport error: {0}")]
    Transport(#[from] reqwest::Error),

    /// OAuth 令牌交换失败。
    #[error("OAuth token exchange failed: {0}")]
    Auth(String),

    /// 百度 API 返回了应用级错误。
    #[error("Baidu API error {code}: {message}")]
    Api {
        /// 百度错误码。
        code: i64,
        /// 百度错误消息。
        message: String,
    },

    /// 响应 JSON 无法解析。
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// 选定的 API 变体尚未实现。
    #[error("Unsupported Baidu API: {0}")]
    UnsupportedApi(BaiduApi),

    /// 图像编码失败。
    #[error("Image encoding failed: {0}")]
    ImageEncoding(String),
}

/// 百度 OCR 操作的便捷 Result 类型。
pub type BaiduResult<T> = std::result::Result<T, BaiduError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_baidu_api_path() {
        assert_eq!(BaiduApi::GeneralBasic.path(), "general_basic");
        assert_eq!(BaiduApi::GeneralAccurate.path(), "accurate_basic");
        assert_eq!(BaiduApi::GeneralBasicWithLocation.path(), "general_basic");
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
        assert_eq!(BaiduApi::TableRecognitionV2.engine_name(), "baidu-table-v2");
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
        assert!(
            !debug.contains("AK-abcdefghijklmnop"),
            "api_key must not appear in Debug"
        );
        assert!(
            !debug.contains("SK-extremely-secret-value"),
            "secret_key must not appear in Debug"
        );
        assert!(
            debug.contains("redacted"),
            "Debug output should contain 'redacted'"
        );
        assert!(debug.contains("BaiduConfig"));
    }
}
