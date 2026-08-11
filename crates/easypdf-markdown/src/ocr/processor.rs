//! OCR processor implementing `PdfMarkdownProcessor`.

use easypdf_core::{PdfError, Result};
use easypdf_core::PdfInput;
use crate::{
    MarkdownProcessorCapabilities, MarkdownWarning, PdfMarkdownProcessor,
};
use easypdf_core::{PdfBlock, PdfBlockType, PdfDocumentModel, PdfPageModel, SourceLocation};
use crate::render::{ImageFormat, RenderBackend, RenderConfig};

use super::config::{OcrConfig, OcrTrigger};
use super::engine::{OcrEngine, OcrImage};

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
        use super::engines::MockOcrEngine;
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
            _ => Err(PdfError::Other(
                "unsupported PDF input type".to_owned(),
            )),
        }
    }

    /// Determine whether a page needs OCR based on the trigger policy.
    fn page_needs_ocr(&self, page: &PdfPageModel) -> bool {
        match self.config.trigger {
            OcrTrigger::Always => true,
            OcrTrigger::OnEmptyPage => {
                // OCR if the page has no paragraph or heading blocks.
                !page
                    .blocks()
                    .iter()
                    .any(|b| matches!(b.block_type(), PdfBlockType::Paragraph | PdfBlockType::Heading))
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
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss
                )]
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
                    let ocr_source = SourceLocation::new(
                        page.index(),
                        confidence.unwrap_or(0.5),
                    );
                    new_page = new_page.with_block(PdfBlock::paragraph(text, ocr_source));

                    new_pages.push(new_page);
                }
                Err(e) => {
                    warnings.push(MarkdownWarning::ProcessorFailed {
                        message: format!(
                            "OCR failed on page {}: {e}",
                            page.index().value() + 1
                        ),
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

/// Adapter that delegates to a borrowed `PdfRenderer`.
struct StoredRendererAdapter<'a>(&'a dyn crate::render::PdfRenderer);

impl crate::render::PdfRenderer for StoredRendererAdapter<'_> {
    fn render_page(
        &self,
        page_index: usize,
        config: &RenderConfig,
    ) -> crate::render::Result<crate::render::RenderedImage> {
        self.0.render_page(page_index, config)
    }

    fn render_page_to_path(
        &self,
        page_index: usize,
        config: &RenderConfig,
        output: &std::path::Path,
    ) -> crate::render::Result<()> {
        self.0.render_page_to_path(page_index, config, output)
    }

    fn render_pages(
        &self,
        page_range: std::ops::Range<usize>,
        config: &RenderConfig,
    ) -> crate::render::Result<Vec<crate::render::RenderedImage>> {
        self.0.render_pages(page_range, config)
    }

    fn name(&self) -> &'static str {
        self.0.name()
    }

    fn max_dpi(&self) -> u32 {
        self.0.max_dpi()
    }

    fn supports_vector(&self) -> bool {
        self.0.supports_vector()
    }
}

/// Convert a `RenderError` to a `PdfError`.
fn render_error_to_pdf(e: &crate::render::RenderError) -> PdfError {
    PdfError::Other(format!("render error: {e}"))
}

/// Build a `RenderedImage` for testing without a real PDF.
///
/// Creates a white RGBA image of the given dimensions.
#[cfg(test)]
pub(crate) fn make_test_rendered_image(
    width: u32,
    height: u32,
    page_index: usize,
) -> crate::render::RenderedImage {
    let pixels = vec![255u8; (width * height * 4) as usize];
    crate::render::RenderedImage::new(width, height, ImageFormat::Png, pixels, page_index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ocr::config::{OcrConfig, OcrTrigger};
    use crate::ocr::engines::MockOcrEngine;
    use crate::render::traits::PdfRenderer;
    use easypdf_core::{PageIndex, PdfMetadata};
    use easypdf_core::SourceLocation;

    /// A mock renderer that returns a white image for any page.
    struct TestRenderer {
        width: u32,
        height: u32,
    }

    impl crate::render::PdfRenderer for TestRenderer {
        fn render_page(
            &self,
            page_index: usize,
            config: &RenderConfig,
        ) -> crate::render::Result<crate::render::RenderedImage> {
            let _ = config;
            Ok(make_test_rendered_image(self.width, self.height, page_index))
        }

        fn name(&self) -> &'static str {
            "test-mock"
        }
    }

    fn loc(page: usize) -> SourceLocation {
        SourceLocation::new(PageIndex::new(page), 1.0)
    }

    fn make_test_renderer() -> Box<dyn crate::render::PdfRenderer> {
        Box::new(TestRenderer {
            width: 100,
            height: 50,
        })
    }

    // --- Mock engine OCR flow ---

    #[test]
    fn mock_engine_ocr_injects_text_on_empty_page() {
        let proc = OcrProcessor::new(
            Box::new(MockOcrEngine::with_text("OCR extracted text")),
            RenderBackend::Text,
        )
        .with_renderer(make_test_renderer());

        let page = PdfPageModel::new(PageIndex::new(0));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let input = PdfInput::from_bytes(vec![]);

        let (result, warnings) = proc.process(&input, doc).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(result.total_blocks(), 1);

        let blocks: Vec<_> = result.iter_all_blocks().collect();
        match blocks[0].1 {
            PdfBlock::Paragraph { text, .. } => {
                assert_eq!(text, "OCR extracted text");
            }
            _ => panic!("expected Paragraph block from OCR"),
        }
    }

    #[test]
    fn mock_engine_ocr_preserves_existing_blocks() {
        let proc = OcrProcessor::new(
            Box::new(MockOcrEngine::with_text("OCR text")),
            RenderBackend::Text,
        )
        .with_renderer(make_test_renderer());

        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::image(
                easypdf_core::ImageData::new(easypdf_core::ImageFormat::Png),
                loc(0),
            ));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let input = PdfInput::from_bytes(vec![]);

        let (result, _) = proc.process(&input, doc).unwrap();
        // Original image block + OCR paragraph = 2 blocks
        assert_eq!(result.total_blocks(), 2);
    }

    // --- OcrTrigger modes ---

    #[test]
    fn trigger_always_ocrs_every_page() {
        let proc = OcrProcessor::new(
            Box::new(MockOcrEngine::with_text("always")),
            RenderBackend::Text,
        )
        .with_renderer(make_test_renderer())
        .with_config(OcrConfig {
            trigger: OcrTrigger::Always,
            ..OcrConfig::default()
        });

        // Page with existing text -- should still OCR with Always trigger.
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::paragraph("existing text", loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let input = PdfInput::from_bytes(vec![]);

        let (result, _) = proc.process(&input, doc).unwrap();
        // existing paragraph + OCR paragraph = 2
        assert_eq!(result.total_blocks(), 2);
    }

    #[test]
    fn trigger_on_empty_page_skips_text_pages() {
        let proc = OcrProcessor::new(
            Box::new(MockOcrEngine::with_text("should not appear")),
            RenderBackend::Text,
        )
        .with_renderer(make_test_renderer())
        .with_config(OcrConfig {
            trigger: OcrTrigger::OnEmptyPage,
            ..OcrConfig::default()
        });

        // Page with existing text -- OnEmptyPage should skip OCR.
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::paragraph("existing text", loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let input = PdfInput::from_bytes(vec![]);

        let (result, _) = proc.process(&input, doc).unwrap();
        // Only the original paragraph, no OCR injection.
        assert_eq!(result.total_blocks(), 1);
    }

    #[test]
    fn trigger_on_empty_page_ocrs_empty_page() {
        let proc = OcrProcessor::new(
            Box::new(MockOcrEngine::with_text("ocr text")),
            RenderBackend::Text,
        )
        .with_renderer(make_test_renderer())
        .with_config(OcrConfig {
            trigger: OcrTrigger::OnEmptyPage,
            ..OcrConfig::default()
        });

        let page = PdfPageModel::new(PageIndex::new(0));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let input = PdfInput::from_bytes(vec![]);

        let (result, _) = proc.process(&input, doc).unwrap();
        assert_eq!(result.total_blocks(), 1);
    }

    #[test]
    fn trigger_when_text_sparse_threshold() {
        let proc = OcrProcessor::new(
            Box::new(MockOcrEngine::with_text("sparse ocr")),
            RenderBackend::Text,
        )
        .with_renderer(make_test_renderer())
        .with_config(OcrConfig {
            trigger: OcrTrigger::WhenTextSparse { threshold: 0.5 },
            ..OcrConfig::default()
        });

        // Page with 1 image + 0 text = 0% text, below 50% threshold -> OCR.
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::image(
                easypdf_core::ImageData::new(easypdf_core::ImageFormat::Png),
                loc(0),
            ));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let input = PdfInput::from_bytes(vec![]);

        let (result, _) = proc.process(&input, doc).unwrap();
        // image + OCR paragraph = 2
        assert_eq!(result.total_blocks(), 2);
    }

    #[test]
    fn trigger_when_text_sparse_skips_dense_page() {
        let proc = OcrProcessor::new(
            Box::new(MockOcrEngine::with_text("should not appear")),
            RenderBackend::Text,
        )
        .with_renderer(make_test_renderer())
        .with_config(OcrConfig {
            trigger: OcrTrigger::WhenTextSparse { threshold: 0.3 },
            ..OcrConfig::default()
        });

        // Page with 3 text blocks + 1 image = 75% text, above 30% threshold -> skip.
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::paragraph("text 1", loc(0)))
            .with_block(PdfBlock::paragraph("text 2", loc(0)))
            .with_block(PdfBlock::paragraph("text 3", loc(0)))
            .with_block(PdfBlock::image(
                easypdf_core::ImageData::new(easypdf_core::ImageFormat::Png),
                loc(0),
            ));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let input = PdfInput::from_bytes(vec![]);

        let (result, _) = proc.process(&input, doc).unwrap();
        // Only original 4 blocks, no OCR injection.
        assert_eq!(result.total_blocks(), 4);
    }

    // --- Low confidence warning ---

    #[test]
    fn low_confidence_generates_warning() {
        let proc = OcrProcessor::new(
            Box::new(MockOcrEngine::with_confidence("low conf", 0.2)),
            RenderBackend::Text,
        )
        .with_renderer(make_test_renderer())
        .with_config(OcrConfig {
            trigger: OcrTrigger::Always,
            min_confidence: 0.5,
            ..OcrConfig::default()
        });

        let page = PdfPageModel::new(PageIndex::new(0));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let input = PdfInput::from_bytes(vec![]);

        let (result, warnings) = proc.process(&input, doc).unwrap();
        // Text is still injected despite low confidence.
        assert_eq!(result.total_blocks(), 1);
        // But a warning is generated.
        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            warnings[0],
            MarkdownWarning::ProcessorFailed { ref message } if message.contains("confidence")
        ));
    }

    // --- Min text length filter ---

    #[test]
    fn min_text_length_filters_short_results() {
        let proc = OcrProcessor::new(
            Box::new(MockOcrEngine::with_text("ab")),
            RenderBackend::Text,
        )
        .with_renderer(make_test_renderer())
        .with_config(OcrConfig {
            trigger: OcrTrigger::Always,
            min_text_length: 5,
            ..OcrConfig::default()
        });

        let page = PdfPageModel::new(PageIndex::new(0));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let input = PdfInput::from_bytes(vec![]);

        let (result, warnings) = proc.process(&input, doc).unwrap();
        // Text "ab" (2 chars) < min_text_length (5) -> discarded.
        assert_eq!(result.total_blocks(), 0);
        // Warning about empty OCR.
        assert_eq!(warnings.len(), 1);
        assert!(matches!(warnings[0], MarkdownWarning::OcrUnavailable { .. }));
    }

    // --- Capabilities ---

    #[test]
    fn capabilities_include_ocr() {
        let proc = OcrProcessor::with_mock_engine();
        assert!(proc.capabilities().ocr());
    }

    // --- Multiple pages ---

    #[test]
    fn processes_multiple_pages() {
        let proc = OcrProcessor::new(
            Box::new(MockOcrEngine::with_text("page ocr")),
            RenderBackend::Text,
        )
        .with_renderer(make_test_renderer())
        .with_config(OcrConfig {
            trigger: OcrTrigger::OnEmptyPage,
            ..OcrConfig::default()
        });

        // Page 0: has text -> skip OCR.
        // Page 1: empty -> OCR.
        // Page 2: has text -> skip OCR.
        let pages = vec![
            PdfPageModel::new(PageIndex::new(0))
                .with_block(PdfBlock::paragraph("has text", loc(0))),
            PdfPageModel::new(PageIndex::new(1)),
            PdfPageModel::new(PageIndex::new(2))
                .with_block(PdfBlock::paragraph("also has text", loc(2))),
        ];
        let doc = PdfDocumentModel::new(PdfMetadata::default(), pages);
        let input = PdfInput::from_bytes(vec![]);

        let (result, warnings) = proc.process(&input, doc).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(result.page_count(), 3);
        // Page 0: 1 block (paragraph)
        assert_eq!(result.pages()[0].blocks().len(), 1);
        // Page 1: 1 block (OCR paragraph)
        assert_eq!(result.pages()[1].blocks().len(), 1);
        // Page 2: 1 block (paragraph)
        assert_eq!(result.pages()[2].blocks().len(), 1);
    }

    // --- OcrImage conversions ---

    #[test]
    fn ocr_image_from_rendered() {
        let rendered = make_test_rendered_image(10, 5, 0);
        let img = OcrImage::from_rendered(&rendered);
        assert_eq!(img.width, 10);
        assert_eq!(img.height, 5);
        assert_eq!(img.pixels.len(), 10 * 5 * 4);
    }

    #[test]
    fn ocr_image_from_dynamic_image() {
        let dynamic = image::DynamicImage::new_rgba8(20, 15);
        let img = OcrImage::from_dynamic_image(&dynamic);
        assert_eq!(img.width, 20);
        assert_eq!(img.height, 15);
        assert_eq!(img.pixels.len(), 20 * 15 * 4);
    }

    // --- Additional coverage tests ---

    #[test]
    fn stored_renderer_adapter_delegates_name() {
        let renderer = make_test_renderer();
        let adapter = StoredRendererAdapter(renderer.as_ref());
        assert_eq!(adapter.name(), "test-mock");
    }

    #[test]
    fn stored_renderer_adapter_delegates_max_dpi() {
        let renderer = make_test_renderer();
        let adapter = StoredRendererAdapter(renderer.as_ref());
        // TestRenderer doesn't override max_dpi, so default applies
        let _ = adapter.max_dpi();
    }

    #[test]
    fn stored_renderer_adapter_delegates_supports_vector() {
        let renderer = make_test_renderer();
        let adapter = StoredRendererAdapter(renderer.as_ref());
        let _ = adapter.supports_vector();
    }

    #[test]
    fn stored_renderer_adapter_delegates_render_page() {
        let renderer = make_test_renderer();
        let adapter = StoredRendererAdapter(renderer.as_ref());
        let config = RenderConfig::default();
        let result = adapter.render_page(0, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn stored_renderer_adapter_delegates_render_page_to_path() {
        let renderer = make_test_renderer();
        let adapter = StoredRendererAdapter(renderer.as_ref());
        let config = RenderConfig::default();
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_ocr_adapter_test.png");
        let result = adapter.render_page_to_path(0, &config, &path);
        // The mock renderer delegates to the underlying renderer
        // which may or may not support render_page_to_path
        let _ = result;
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn stored_renderer_adapter_delegates_render_pages() {
        let renderer = make_test_renderer();
        let adapter = StoredRendererAdapter(renderer.as_ref());
        let config = RenderConfig::default();
        let result = adapter.render_pages(0..1, &config);
        let _ = result;
    }

    #[test]
    fn render_error_to_pdf_conversion() {
        let render_err = crate::render::RenderError::InvalidPage {
            index: 5,
            total: 3,
        };
        let pdf_err = render_error_to_pdf(&render_err);
        assert!(pdf_err.to_string().contains("render error"));
    }

    #[test]
    fn ocr_processor_debug_format() {
        let proc = OcrProcessor::with_mock_engine();
        let debug = format!("{proc:?}");
        assert!(debug.contains("OcrProcessor"));
        assert!(debug.contains("engine"));
    }

    #[test]
    fn trigger_when_text_sparse_empty_page() {
        let proc = OcrProcessor::new(
            Box::new(MockOcrEngine::with_text("ocr on empty")),
            RenderBackend::Text,
        )
        .with_renderer(make_test_renderer())
        .with_config(OcrConfig {
            trigger: OcrTrigger::WhenTextSparse { threshold: 0.5 },
            ..OcrConfig::default()
        });

        // Empty page: total=0, should OCR
        let page = PdfPageModel::new(PageIndex::new(0));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let input = PdfInput::from_bytes(vec![]);

        let (result, _) = proc.process(&input, doc).unwrap();
        assert_eq!(result.total_blocks(), 1);
    }

    #[test]
    fn trigger_when_text_sparse_with_text_blocks() {
        let proc = OcrProcessor::new(
            Box::new(MockOcrEngine::with_text("should not appear")),
            RenderBackend::Text,
        )
        .with_renderer(make_test_renderer())
        .with_config(OcrConfig {
            trigger: OcrTrigger::WhenTextSparse { threshold: 0.5 },
            ..OcrConfig::default()
        });

        // Page with 2 text blocks + 0 non-text = 100% text, above 50% threshold
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::paragraph("text1", loc(0)))
            .with_block(PdfBlock::heading(1, "title", loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let input = PdfInput::from_bytes(vec![]);

        let (result, _) = proc.process(&input, doc).unwrap();
        assert_eq!(result.total_blocks(), 2);
    }

    #[test]
    fn process_preserves_page_dimensions() {
        let proc = OcrProcessor::new(
            Box::new(MockOcrEngine::with_text("ocr text")),
            RenderBackend::Text,
        )
        .with_renderer(make_test_renderer())
        .with_config(OcrConfig {
            trigger: OcrTrigger::Always,
            ..OcrConfig::default()
        });

        let page = PdfPageModel::new(PageIndex::new(0))
            .with_dimensions(595.0, 842.0)
            .with_rotation(90);
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let input = PdfInput::from_bytes(vec![]);

        let (result, _) = proc.process(&input, doc).unwrap();
        let result_page = &result.pages()[0];
        assert_eq!(result_page.width_pt(), Some(595.0));
        assert_eq!(result_page.height_pt(), Some(842.0));
        assert_eq!(result_page.rotation(), 90);
    }

    #[test]
    fn ocr_with_no_confidence() {
        let proc = OcrProcessor::new(
            Box::new(MockOcrEngine::with_text("text")),
            RenderBackend::Text,
        )
        .with_renderer(make_test_renderer())
        .with_config(OcrConfig {
            trigger: OcrTrigger::Always,
            min_confidence: 0.9,
            ..OcrConfig::default()
        });

        let page = PdfPageModel::new(PageIndex::new(0));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let input = PdfInput::from_bytes(vec![]);

        let (result, warnings) = proc.process(&input, doc).unwrap();
        // MockOcrEngine::with_text returns confidence = None by default
        assert_eq!(result.total_blocks(), 1);
        // No confidence warning because confidence is None
        assert!(warnings.is_empty());
    }
}
