//! Request building trait for HTTP-based OCR engines.
//!
//! Each OCR engine implements [`OcrRequest`] to describe how to construct
//! the JSON body and extra headers for its specific API.

use std::collections::HashMap;

use easypdf_markdown::ocr::OcrImage;

use super::auth::AuthMethod;
use super::error::Result;

/// Configuration passed to request builders.
#[derive(Debug, Clone, Default)]
pub struct RequestConfig {
    /// Optional image encoding format hint.
    pub image_format: Option<String>,
    /// Optional language hint for the OCR engine.
    pub language: Option<String>,
}

/// Trait for building HTTP request bodies for an OCR engine.
///
/// Each cloud OCR provider has a different request format. Implementors
/// describe the endpoint, authentication, and how to construct the JSON
/// body from an image.
///
/// # Implementing
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
    /// The API endpoint URL.
    fn endpoint(&self) -> &str;

    /// The authentication method for this endpoint.
    fn auth(&self) -> &AuthMethod;

    /// Build the JSON request body from the image and configuration.
    ///
    /// # Errors
    ///
    /// Returns `OcrHttpError` if the image cannot be encoded or the
    /// body cannot be constructed.
    fn build_request_body(
        &self,
        image: &OcrImage,
        config: &RequestConfig,
    ) -> Result<serde_json::Value>;

    /// Extra HTTP headers to include in the request.
    ///
    /// Override this to add engine-specific headers beyond the default
    /// `Content-Type: application/json` and authentication headers.
    fn extra_headers(&self) -> HashMap<String, String> {
        HashMap::new()
    }

    /// Human-readable name of this OCR engine (e.g., `"glm-ocr"`).
    fn engine_name(&self) -> &'static str;

    /// Supported language codes.
    fn languages(&self) -> &[&str];
}
