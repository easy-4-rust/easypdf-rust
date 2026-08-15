//! 基于 HTTP 的 OCR 引擎请求构建 trait。
//!
//! 每个 OCR 引擎实现 [`OcrRequest`] 来描述如何为其专用 API
//! 构造 JSON 请求体和额外请求头。

use std::collections::HashMap;

use easypdf_markdown::ocr::OcrImage;

use super::auth::AuthMethod;
use super::error::Result;

/// 传递给请求构建器的配置。
#[derive(Debug, Clone, Default)]
pub struct RequestConfig {
    /// 可选的图像编码格式提示。
    pub image_format: Option<String>,
    /// 可选的 OCR 引擎语言提示。
    pub language: Option<String>,
}

/// 为 OCR 引擎构建 HTTP 请求体的 trait。
///
/// 每个云 OCR 提供商有不同的请求格式。实现者描述端点、认证方式
/// 以及如何从图像构造 JSON 请求体。
///
/// # 实现示例
///
/// ```ignore
/// use easypdf_ocr::http::{
///     OcrRequest, AuthMethod, RequestConfig,
///     error::Result,
/// };
/// use easypdf_markdown::ocr::OcrImage;
/// use serde_json::Value;
/// use std::collections::HashMap;
///
/// struct GlmOcrRequest;
///
/// impl OcrRequest for GlmOcrRequest {
///     fn endpoint(&self) -> &str {
///         "https://open.bigmodel.cn/api/paas/v4/chat/completions"
///     }
///
///     fn auth(&self) -> &AuthMethod {
///         // Return a reference to stored auth (e.g., API key bearer token)
///         static AUTH: std::sync::OnceLock<AuthMethod> = std::sync::OnceLock::new();
///         AUTH.get_or_init(|| AuthMethod::Bearer("sk-example".into()))
///     }
///
///     fn build_request_body(
///         &self,
///         image: &OcrImage,
///         config: &RequestConfig,
///     ) -> Result<Value> {
///         // Build the engine-specific JSON body
///         Ok(serde_json::json!({
///             "model": "glm-4v",
///             "messages": [{ "role": "user", "content": "extract text" }],
///         }))
///     }
/// }
/// ```
pub trait OcrRequest: Send + Sync {
    /// API 端点 URL。
    fn endpoint(&self) -> &str;

    /// 此端点的认证方式。
    fn auth(&self) -> &AuthMethod;

    /// 从图像和配置构建 JSON 请求体。
    ///
    /// # Errors
    ///
    /// 若图像无法编码或请求体无法构造，返回 `OcrHttpError`。
    fn build_request_body(
        &self,
        image: &OcrImage,
        config: &RequestConfig,
    ) -> Result<serde_json::Value>;

    /// 请求中包含的额外 HTTP 请求头。
    ///
    /// 重写此方法以添加超出默认 `Content-Type: application/json`
    /// 和认证请求头之外的引擎专用请求头。
    fn extra_headers(&self) -> HashMap<String, String> {
        HashMap::new()
    }

    /// 此 OCR 引擎的人类可读名称（如 `"glm-ocr"`）。
    fn engine_name(&self) -> &'static str;

    /// 支持的语言代码。
    fn languages(&self) -> &[&str];
}
