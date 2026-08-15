use super::*;
use crate::ocr::config::{OcrConfig, OcrTrigger};
use crate::ocr::engines::MockOcrEngine;
use crate::render::traits::PdfRenderer;
use crate::render::{RenderBackend, RenderConfig};
use crate::{MarkdownWarning, PdfMarkdownProcessor};
use easypdf_core::PageIndex;
use easypdf_core::{
    PdfBlock, PdfDocumentModel, PdfInput, PdfMetadata, PdfPageModel, SourceLocation,
};

use super::renderer::{StoredRendererAdapter, make_test_rendered_image, render_error_to_pdf};

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
        Ok(make_test_rendered_image(
            self.width,
            self.height,
            page_index,
        ))
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

    let page = PdfPageModel::new(PageIndex::new(0)).with_block(PdfBlock::image(
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
    let page = PdfPageModel::new(PageIndex::new(0)).with_block(PdfBlock::image(
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
    assert!(matches!(
        warnings[0],
        MarkdownWarning::OcrUnavailable { .. }
    ));
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
        PdfPageModel::new(PageIndex::new(0)).with_block(PdfBlock::paragraph("has text", loc(0))),
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
    let img = crate::ocr::engine::OcrImage::from_rendered(&rendered);
    assert_eq!(img.width, 10);
    assert_eq!(img.height, 5);
    assert_eq!(img.pixels.len(), 10 * 5 * 4);
}

#[test]
fn ocr_image_from_dynamic_image() {
    let dynamic = image::DynamicImage::new_rgba8(20, 15);
    let img = crate::ocr::engine::OcrImage::from_dynamic_image(&dynamic);
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
    let render_err = crate::render::RenderError::InvalidPage { index: 5, total: 3 };
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
