//! Response parsing trait for HTTP-based OCR engines.
//!
//! Each OCR engine implements [`OcrResponseParser`] to describe how to
//! extract text, confidence, and word boxes from its specific JSON response.

use easypdf_markdown::ocr::OcrResult;

use super::error::Result;

/// Trait for parsing OCR engine JSON responses.
///
/// Each cloud OCR provider returns results in a different format.
/// Implementors parse the raw JSON into a standardized [`OcrResult`].
///
/// # Implementing
///
/// ```ignore
/// use easypdf_ocr::http::{
///     OcrResponseParser,
///     error::Result,
/// };
/// use easypdf_markdown::ocr::OcrResult;
///
/// struct GlmOcrParser;
///
/// impl OcrResponseParser for GlmOcrParser {
///     fn parse_response(&self, raw: &serde_json::Value) -> Result<OcrResult> {
///         // Extract text from engine-specific JSON structure
///         let text = raw["data"]["text"].as_str().unwrap_or("");
///         Ok(OcrResult { text: text.to_string(), ..Default::default() })
///     }
/// }
/// ```
pub trait OcrResponseParser: Send + Sync {
    /// Parse the raw JSON response into an [`OcrResult`].
    ///
    /// # Errors
    ///
    /// Returns `OcrHttpError::InvalidResponse` if the response structure
    /// does not match expectations, or `OcrHttpError::Engine` if the
    /// engine returned an application-level error.
    fn parse_response(&self, raw: &serde_json::Value) -> Result<OcrResult>;
}
