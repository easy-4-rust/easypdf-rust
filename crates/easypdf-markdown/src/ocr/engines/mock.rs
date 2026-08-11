//! Mock OCR engine for testing and default compilation.

use easypdf_core::CapabilityLevel;

use crate::ocr::engine::{OcrEngine, OcrImage, OcrResult};

/// Mock OCR engine that returns fixed text.
///
/// Always available (no feature gate). Useful for:
/// - Unit and integration tests
/// - Default compilation without OCR dependencies
/// - Placeholder in the processor pipeline
///
/// # Examples
///
/// ```
/// use easypdf_markdown::ocr::{OcrEngine, OcrImage, engines::MockOcrEngine};
///
/// let engine = MockOcrEngine::new();
/// let img = OcrImage::new(100, 50, vec![0u8; 100 * 50 * 4]);
/// let result = engine.recognize(&img).unwrap();
/// assert!(!result.text.is_empty());
/// ```
#[derive(Debug, Clone)]
pub struct MockOcrEngine {
    fixed_text: String,
    confidence: f32,
}

impl MockOcrEngine {
    /// Create a mock engine with default text ("[OCR text extracted by mock engine]").
    #[must_use]
    pub fn new() -> Self {
        Self {
            fixed_text: "[OCR text extracted by mock engine]".to_owned(),
            confidence: 1.0,
        }
    }

    /// Create a mock engine that returns the given text.
    #[must_use]
    pub fn with_text(text: impl Into<String>) -> Self {
        Self {
            fixed_text: text.into(),
            confidence: 1.0,
        }
    }

    /// Create a mock engine with a specific confidence score.
    #[must_use]
    pub fn with_confidence(text: impl Into<String>, confidence: f32) -> Self {
        Self {
            fixed_text: text.into(),
            confidence: confidence.clamp(0.0, 1.0),
        }
    }
}

impl Default for MockOcrEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl OcrEngine for MockOcrEngine {
    fn recognize(
        &self,
        _image: &OcrImage,
    ) -> std::result::Result<OcrResult, Box<dyn std::error::Error + Send + Sync>> {
        Ok(OcrResult {
            text: self.fixed_text.clone(),
            confidence: Some(self.confidence),
            word_boxes: vec![],
        })
    }

    fn name(&self) -> &'static str {
        "mock"
    }

    fn languages(&self) -> &[&str] {
        &["en"]
    }

    fn level(&self) -> CapabilityLevel {
        CapabilityLevel::Heuristic
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_engine_returns_fixed_text() {
        let engine = MockOcrEngine::new();
        let img = OcrImage::new(10, 10, vec![0u8; 400]);
        let result = engine.recognize(&img).unwrap();
        assert_eq!(result.text, "[OCR text extracted by mock engine]");
        assert_eq!(result.confidence, Some(1.0));
        assert!(result.word_boxes.is_empty());
    }

    #[test]
    fn mock_engine_with_custom_text() {
        let engine = MockOcrEngine::with_text("Hello World");
        let img = OcrImage::new(10, 10, vec![0u8; 400]);
        let result = engine.recognize(&img).unwrap();
        assert_eq!(result.text, "Hello World");
    }

    #[test]
    fn mock_engine_with_custom_confidence() {
        let engine = MockOcrEngine::with_confidence("text", 0.3);
        let img = OcrImage::new(10, 10, vec![0u8; 400]);
        let result = engine.recognize(&img).unwrap();
        assert_eq!(result.confidence, Some(0.3));
    }

    #[test]
    fn mock_engine_metadata() {
        let engine = MockOcrEngine::new();
        assert_eq!(engine.name(), "mock");
        assert_eq!(engine.languages(), &["en"]);
        assert_eq!(engine.level(), CapabilityLevel::Heuristic);
    }
}
