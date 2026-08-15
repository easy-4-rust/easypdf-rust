use crate::render::{ImageFormat, RenderBackend, RenderConfig};
use crate::{MarkdownProcessorCapabilities, MarkdownWarning, PdfMarkdownProcessor};
use easypdf_core::PdfInput;
use easypdf_core::{PdfBlock, PdfBlockType, PdfDocumentModel, PdfPageModel, SourceLocation};
use easypdf_core::{PdfError, Result};

use crate::ocr::config::{OcrConfig, OcrTrigger};
use crate::ocr::engine::{OcrEngine, OcrImage};

use super::renderer::{StoredRendererAdapter, render_error_to_pdf};

/// OCR processor for the markdown processor pipeline.
///
/// Scans the document model for pages that need OCR based on the configured
/// [`OcrTrigger`] policy. Renders those pages to images, runs OCR via the
/// configured [`OcrEngine`], and injects the recognized text as new
/// [`PdfBlock::Paragraph`] blocks.
///
/// # Examples
///
/// ```
/// use easypdf_markdown::PdfMarkdownProcessor;
/// use easypdf_markdown::ocr::OcrProcessor;
///
/// let processor = OcrProcessor::with_mock_engine();
/// let caps = processor.capabilities();
/// assert!(caps.ocr());
/// ```
pub struct OcrProcessor {
    engine: Box<dyn OcrEngine>,
    renderer: Option<Box<dyn crate::render::PdfRenderer>>,
    backend: RenderBackend,
    config: OcrConfig,
}

impl std::fmt::Debug for OcrProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OcrProcessor")
            .field("engine", &self.engine.name())
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl OcrProcessor {
    /// Create a new OCR processor with the given engine and render backend.
    ///
    /// The render backend is used to build a [`PdfRenderer`](crate::render::PdfRenderer)
    /// from the PDF input path when processing.
    #[must_use]
    pub fn new(engine: Box<dyn OcrEngine>, backend: RenderBackend) -> Self {
        Self {
            engine,
            renderer: None,
            backend,
            config: OcrConfig::default(),
        }
    }

    /// Create a new OCR processor with a mock engine (for testing).
    #[must_use]
    pub fn with_mock_engine() -> Self {
        use crate::ocr::engines::MockOcrEngine;
        Self::new(Box::new(MockOcrEngine::new()), RenderBackend::Text)
    }

    /// Set a pre-built renderer (overrides the backend for rendering).
    ///
    /// Useful for testing with a mock renderer that does not require a real PDF file.
    #[must_use]
    pub fn with_renderer(mut self, renderer: Box<dyn crate::render::PdfRenderer>) -> Self {
        self.renderer = Some(renderer);
        self
    }

    /// Set the OCR configuration.
    #[must_use]
    pub fn with_config(mut self, config: OcrConfig) -> Self {
        self.config = config;
        self
    }

    /// Get the renderer to use for page rendering.
    ///
    /// Uses the pre-built renderer if available, otherwise builds from the input path.
    /// For bytes input with no pre-built renderer, writes to a temp file.
    fn get_renderer<'a>(
        &'a self,
        input: &PdfInput,
    ) -> Result<Box<dyn crate::render::PdfRenderer + 'a>> {
        if let Some(ref renderer) = self.renderer {
            // A pre-built renderer was injected (e.g., for testing).
            // Wrap it in a delegating adapter since we can't clone trait objects.
            return Ok(Box::new(StoredRendererAdapter(renderer.as_ref())));
        }

        match input {
            PdfInput::Path(path) => self
                .backend
                .build_renderer(path)
                .map_err(|e| render_error_to_pdf(&e)),
            PdfInput::Bytes(bytes) => {
                let tmp = tempfile::NamedTempFile::new()?;
                std::fs::write(tmp.path(), bytes)?;
                self.backend
                    .build_renderer(tmp.path())
                    .map_err(|e| render_error_to_pdf(&e))
            }
            _ => Err(PdfError::Other("unsupported PDF input type".to_owned())),
        }
    }

    /// Determine whether a page needs OCR based on the trigger policy.
    fn page_needs_ocr(&self, page: &PdfPageModel) -> bool {
        match self.config.trigger {
            OcrTrigger::Always => true,
            OcrTrigger::OnEmptyPage => {
                // OCR if the page has no paragraph or heading blocks.
                !page.blocks().iter().any(|b| {
                    matches!(
                        b.block_type(),
                        PdfBlockType::Paragraph | PdfBlockType::Heading
                    )
                })
            }
            OcrTrigger::WhenTextSparse { threshold } => {
                let total = page.blocks().len();
                if total == 0 {
                    return true;
                }
                let text_count = page
                    .blocks()
                    .iter()
                    .filter(|b| {
                        matches!(
                            b.block_type(),
                            PdfBlockType::Paragraph
                                | PdfBlockType::Heading
                                | PdfBlockType::Code
                                | PdfBlockType::Footnote
                                | PdfBlockType::BlockQuote
                        )
                    })
                    .count();
                // Threshold comparison: text_count / total < threshold.
                // Rewrite as integer: text_count * 1000 < total * (threshold * 1000).
                // Page counts are always small, so multiplication won't overflow.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let threshold_permille = (f64::from(threshold) * 1000.0).round() as usize;
                text_count.saturating_mul(1000) < total.saturating_mul(threshold_permille)
            }
        }
    }

    /// Render a page and run OCR on it.
    fn ocr_page(
        &self,
        renderer: &dyn crate::render::PdfRenderer,
        page_index: usize,
    ) -> Result<(String, Option<f32>)> {
        let render_config = RenderConfig {
            dpi: self.config.render_dpi,
            format: ImageFormat::Png,
            ..RenderConfig::default()
        };

        let rendered_img = renderer
            .render_page(page_index, &render_config)
            .map_err(|e| render_error_to_pdf(&e))?;
        let ocr_image = OcrImage::from_rendered(&rendered_img);

        let ocr_result = self
            .engine
            .recognize(&ocr_image)
            .map_err(|e| PdfError::Other(format!("OCR engine error: {e}")))?;

        let text = ocr_result.text;
        let confidence = ocr_result.confidence;

        // Filter by minimum text length.
        if text.trim().len() < self.config.min_text_length {
            return Ok((String::new(), confidence));
        }

        Ok((text, confidence))
    }
}

impl PdfMarkdownProcessor for OcrProcessor {
    fn capabilities(&self) -> MarkdownProcessorCapabilities {
        MarkdownProcessorCapabilities::new().with_ocr()
    }

    fn process(
        &self,
        input: &PdfInput,
        document: PdfDocumentModel,
    ) -> Result<(PdfDocumentModel, Vec<MarkdownWarning>)> {
        let renderer = self.get_renderer(input)?;
        let mut new_pages = Vec::with_capacity(document.page_count());
        let mut warnings = Vec::new();

        for page in document.pages() {
            if !self.page_needs_ocr(page) {
                new_pages.push(page.clone());
                continue;
            }

            match self.ocr_page(renderer.as_ref(), page.index().value()) {
                Ok((text, confidence)) => {
                    if text.is_empty() {
                        // OCR returned no usable text.
                        warnings.push(MarkdownWarning::OcrUnavailable {
                            page_index: page.index(),
                        });
                        new_pages.push(page.clone());
                        continue;
                    }

                    // Check confidence threshold.
                    if let Some(conf) = confidence
                        && conf < self.config.min_confidence
                    {
                        warnings.push(MarkdownWarning::ProcessorFailed {
                            message: format!(
                                "OCR confidence {conf:.2} below threshold {:.2} on page {}",
                                self.config.min_confidence,
                                page.index().value() + 1
                            ),
                        });
                    }

                    // Build new page with OCR text injected.
                    let mut new_page = PdfPageModel::new(page.index());
                    if let (Some(w), Some(h)) = (page.width_pt(), page.height_pt()) {
                        new_page = new_page.with_dimensions(w, h);
                    }
                    new_page = new_page.with_rotation(page.rotation());

                    // Preserve existing blocks.
                    for block in page.blocks() {
                        new_page = new_page.with_block(block.clone());
                    }

                    // Inject OCR text as a new paragraph.
                    let ocr_source = SourceLocation::new(page.index(), confidence.unwrap_or(0.5));
                    new_page = new_page.with_block(PdfBlock::paragraph(text, ocr_source));

                    new_pages.push(new_page);
                }
                Err(e) => {
                    warnings.push(MarkdownWarning::ProcessorFailed {
                        message: format!("OCR failed on page {}: {e}", page.index().value() + 1),
                    });
                    new_pages.push(page.clone());
                }
            }
        }

        Ok((
            PdfDocumentModel::new(document.metadata().clone(), new_pages),
            warnings,
        ))
    }
}
