//! OCR configuration and trigger policies.

/// Configuration for the OCR processor.
///
/// Controls rendering DPI, when OCR is triggered, and quality thresholds.
///
/// # Examples
///
/// ```
/// use easypdf_markdown::ocr::{OcrConfig, OcrTrigger};
///
/// let config = OcrConfig {
///     render_dpi: 300,
///     trigger: OcrTrigger::Always,
///     min_confidence: 0.8,
///     ..OcrConfig::default()
/// };
/// assert_eq!(config.render_dpi, 300);
/// ```
#[derive(Debug, Clone)]
pub struct OcrConfig {
    /// Rendering DPI for page-to-image conversion. Default: 200.
    ///
    /// Higher values improve OCR accuracy but increase memory usage and
    /// processing time. 200 DPI is a good balance for most documents.
    pub render_dpi: u32,

    /// When to trigger OCR. Default: [`OcrTrigger::OnEmptyPage`].
    pub trigger: OcrTrigger,

    /// Minimum text length to keep from OCR results. Default: 0 (keep all).
    ///
    /// OCR results shorter than this threshold are discarded as noise.
    pub min_text_length: usize,

    /// Minimum confidence threshold. Default: 0.5.
    ///
    /// OCR results with confidence below this value are flagged with a warning.
    pub min_confidence: f32,
}

impl Default for OcrConfig {
    fn default() -> Self {
        Self {
            render_dpi: 200,
            trigger: OcrTrigger::OnEmptyPage,
            min_text_length: 0,
            min_confidence: 0.5,
        }
    }
}

/// When to trigger OCR processing on a page.
///
/// # Examples
///
/// ```
/// use easypdf_markdown::ocr::OcrTrigger;
///
/// let trigger = OcrTrigger::WhenTextSparse { threshold: 0.3 };
/// assert!(matches!(trigger, OcrTrigger::WhenTextSparse { threshold } if threshold == 0.3));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[non_exhaustive]
pub enum OcrTrigger {
    /// Always attempt OCR on every page.
    Always,

    /// Only OCR when a page has no extractable native text blocks.
    ///
    /// This is the default and the primary use case: scanned PDFs where the
    /// text extractor returns empty pages.
    #[default]
    OnEmptyPage,

    /// OCR when the ratio of text blocks to total blocks is below the threshold.
    ///
    /// The `threshold` is a value in `0.0..=1.0`. A page is considered text-sparse
    /// when `(text_block_count / total_block_count) < threshold`.
    WhenTextSparse {
        /// Text-to-total block ratio threshold (0.0 to 1.0).
        threshold: f32,
    },
}
