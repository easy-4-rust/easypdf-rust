//! OCR engine abstraction and result types.

use crate::render::RenderedImage;
use easypdf_core::CapabilityLevel;

/// Input image for OCR recognition.
///
/// Holds raw RGBA pixel data along with dimensions. Construct from a
/// [`RenderedImage`] or an `image::DynamicImage`.
///
/// # Examples
///
/// ```
/// use easypdf_markdown::ocr::OcrImage;
/// use image::DynamicImage;
///
/// let img = DynamicImage::new_rgba8(100, 50);
/// let ocr_img = OcrImage::from_dynamic_image(&img);
/// assert_eq!(ocr_img.width, 100);
/// assert_eq!(ocr_img.height, 50);
/// ```
#[derive(Debug, Clone)]
pub struct OcrImage {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Raw RGBA pixel data (4 bytes per pixel, row-major, top-to-bottom).
    pub pixels: Vec<u8>,
}

impl OcrImage {
    /// Create an `OcrImage` from a rendered PDF page image.
    ///
    /// # Panics
    ///
    /// Panics if the pixel buffer length does not equal `width * height * 4`.
    #[must_use]
    pub fn from_rendered(rendered: &RenderedImage) -> Self {
        Self {
            width: rendered.width,
            height: rendered.height,
            pixels: rendered.pixels.clone(),
        }
    }

    /// Create an `OcrImage` from an `image::DynamicImage`.
    ///
    /// Converts to RGBA8 format internally.
    #[must_use]
    pub fn from_dynamic_image(image: &image::DynamicImage) -> Self {
        let rgba = image.to_rgba8();
        Self {
            width: rgba.width(),
            height: rgba.height(),
            pixels: rgba.into_raw(),
        }
    }

    /// Create an `OcrImage` from raw RGBA pixel data.
    #[must_use]
    pub const fn new(width: u32, height: u32, pixels: Vec<u8>) -> Self {
        Self {
            width,
            height,
            pixels,
        }
    }
}

/// Result of OCR text recognition.
///
/// Contains the extracted text, optional confidence score, and optional
/// word-level bounding boxes for spatial layout preservation.
#[derive(Debug, Clone)]
pub struct OcrResult {
    /// Extracted text from the image.
    pub text: String,
    /// Overall confidence score (0.0 to 1.0), if provided by the engine.
    pub confidence: Option<f32>,
    /// Per-word bounding boxes, if provided by the engine.
    pub word_boxes: Vec<WordBox>,
}

/// Bounding box for a single recognized word.
///
/// Coordinates are in pixels relative to the input image origin (top-left).
#[derive(Debug, Clone)]
pub struct WordBox {
    /// The recognized word text.
    pub text: String,
    /// X coordinate of the top-left corner (pixels).
    pub x: u32,
    /// Y coordinate of the top-left corner (pixels).
    pub y: u32,
    /// Width of the bounding box (pixels).
    pub width: u32,
    /// Height of the bounding box (pixels).
    pub height: u32,
    /// Per-word confidence score (0.0 to 1.0), if available.
    pub confidence: Option<f32>,
}

/// OCR engine abstraction.
///
/// Implementors provide text recognition from images using different backends
/// (local ML models, cloud APIs, mocks). The trait is object-safe and requires
/// `Send + Sync` for use across threads.
///
/// # Implementing a custom engine
///
/// ```
/// use easypdf_markdown::ocr::{OcrEngine, OcrImage, OcrResult};
/// use easypdf_core::CapabilityLevel;
///
/// struct MyEngine;
///
/// impl OcrEngine for MyEngine {
///     fn recognize(&self, image: &OcrImage) -> std::result::Result<OcrResult, Box<dyn std::error::Error + Send + Sync>> {
///         Ok(OcrResult {
///             text: format!("OCR of {}x{} image", image.width, image.height),
///             confidence: Some(0.95),
///             word_boxes: vec![],
///         })
///     }
///
///     fn name(&self) -> &'static str { "my-engine" }
///     fn languages(&self) -> &[&str] { &["en"] }
///     fn level(&self) -> CapabilityLevel { CapabilityLevel::Heuristic }
/// }
/// ```
pub trait OcrEngine: Send + Sync {
    /// Perform OCR on the given image and return recognized text.
    ///
    /// # Errors
    ///
    /// Returns an error if the engine fails to process the image (model load
    /// failure, network timeout, unsupported format, etc.).
    fn recognize(
        &self,
        image: &OcrImage,
    ) -> std::result::Result<OcrResult, Box<dyn std::error::Error + Send + Sync>>;

    /// Human-readable name of this OCR engine (e.g., "ocrs", "llm-gpt-4o").
    fn name(&self) -> &'static str;

    /// Supported language codes (e.g., `["en", "zh"]`).
    fn languages(&self) -> &[&str];

    /// Capability level of this engine.
    ///
    /// - [`CapabilityLevel::Heuristic`]: local ML model (e.g., ocrs)
    /// - [`CapabilityLevel::Cloud`]: cloud API (e.g., LLM Vision)
    fn level(&self) -> CapabilityLevel;
}
