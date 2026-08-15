//! Pure Rust OCR backend using the `ocrs` crate.
//!
//! Requires the `ocrs` feature flag. Uses the `ocrs` crate's built-in
//! detection and recognition ONNX models for text extraction.

use easypdf_core::CapabilityLevel;

use crate::ocr::engine::{OcrEngine, OcrImage, OcrResult, WordBox};

/// Pure Rust OCR engine backed by the `ocrs` crate.
///
/// Uses ONNX-based text detection and recognition models. No external
/// system dependencies are required.
///
/// # Examples
///
/// ```no_run
/// use easypdf_markdown::ocr::{OcrEngine, engines::OcrsEngine};
///
/// let engine = OcrsEngine::new().expect("failed to initialize ocrs");
/// println!("engine: {}", engine.name());
/// ```
pub struct OcrsEngine {
    engine: ocrs::OcrEngine,
}

impl std::fmt::Debug for OcrsEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OcrsEngine")
            .field("engine", &"<ocrs::OcrEngine>")
            .finish()
    }
}

impl OcrsEngine {
    /// Create a new ocrs engine with default model parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if the ONNX models cannot be loaded or initialized.
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let params = ocrs::OcrEngineParams::default();
        let engine = ocrs::OcrEngine::new(params)?;
        Ok(Self { engine })
    }

    /// Create a new ocrs engine with custom parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if the ONNX models cannot be loaded or initialized.
    pub fn with_params(
        params: ocrs::OcrEngineParams,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let engine = ocrs::OcrEngine::new(params)?;
        Ok(Self { engine })
    }
}

impl OcrEngine for OcrsEngine {
    fn recognize(
        &self,
        image: &OcrImage,
    ) -> std::result::Result<OcrResult, Box<dyn std::error::Error + Send + Sync>> {
        // Convert RGBA pixels to RGB for ocrs (HWC order).
        let rgb_pixels: Vec<u8> = image
            .pixels
            .chunks_exact(4)
            .flat_map(|rgba| &rgba[..3])
            .copied()
            .collect();

        let img_source = ocrs::ImageSource::from_bytes(&rgb_pixels, (image.width, image.height))?;
        let ocr_input = self.engine.prepare_input(img_source)?;

        // Use the convenience API: detect + recognize + collect as string.
        let text = self.engine.get_text(&ocr_input)?;

        // For word-level details, detect words and build bounding boxes.
        let word_boxes = match self.engine.detect_words(&ocr_input) {
            Ok(rects) => rects
                .iter()
                .map(|r| {
                    // ocrs 的坐标是 f32 像素值，转 u32 时截断/去符号是可接受的精度损失。
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let (cx, cy, rw, rh) = (
                        r.center().x as u32,
                        r.center().y as u32,
                        r.width() as u32,
                        r.height() as u32,
                    );
                    WordBox {
                        text: String::new(), // ocrs doesn't provide per-word text
                        x: cx,
                        y: cy,
                        width: rw,
                        height: rh,
                        confidence: None,
                    }
                })
                .collect(),
            Err(_) => Vec::new(),
        };

        Ok(OcrResult {
            text,
            confidence: None,
            word_boxes,
        })
    }

    fn name(&self) -> &'static str {
        "ocrs"
    }

    fn languages(&self) -> &[&str] {
        &["en"]
    }

    fn level(&self) -> CapabilityLevel {
        CapabilityLevel::Heuristic
    }
}
