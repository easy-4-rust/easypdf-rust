use crate::render::{ImageFormat, RenderBackend, RenderConfig};
use crate::{MarkdownProcessorCapabilities, MarkdownWarning, PdfMarkdownProcessor};
use easypdf_core::PdfInput;
use easypdf_core::{PdfBlock, PdfBlockType, PdfDocumentModel, PdfPageModel, SourceLocation};
use easypdf_core::{PdfError, Result};

use crate::ocr::config::{OcrConfig, OcrTrigger};
use crate::ocr::engine::{OcrEngine, OcrImage};

use super::renderer::{StoredRendererAdapter, render_error_to_pdf};

/// Markdown 处理器管道中的 OCR 处理器。
///
/// 扫描文档模型中需要 OCR 的页面（基于配置的 [`OcrTrigger`] 策略），
/// 将这些页面渲染为图像，通过配置的 [`OcrEngine`] 执行 OCR，
/// 并将识别文本作为新的 [`PdfBlock::Paragraph`] 块注入。
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
    /// 使用给定引擎和渲染后端创建新的 OCR 处理器。
    ///
    /// 渲染后端用于在处理时从 PDF 输入路径构建
    /// [`PdfRenderer`](crate::render::PdfRenderer)。
    #[must_use]
    pub fn new(engine: Box<dyn OcrEngine>, backend: RenderBackend) -> Self {
        Self {
            engine,
            renderer: None,
            backend,
            config: OcrConfig::default(),
        }
    }

    /// 使用模拟引擎创建新的 OCR 处理器（用于测试）。
    #[must_use]
    pub fn with_mock_engine() -> Self {
        use crate::ocr::engines::MockOcrEngine;
        Self::new(Box::new(MockOcrEngine::new()), RenderBackend::Text)
    }

    /// 设置预构建的渲染器（覆盖后端渲染方式）。
    ///
    /// 适用于使用不需要真实 PDF 文件的模拟渲染器进行测试。
    #[must_use]
    pub fn with_renderer(mut self, renderer: Box<dyn crate::render::PdfRenderer>) -> Self {
        self.renderer = Some(renderer);
        self
    }

    /// 设置 OCR 配置。
    #[must_use]
    pub fn with_config(mut self, config: OcrConfig) -> Self {
        self.config = config;
        self
    }

    /// 获取用于页面渲染的渲染器。
    ///
    /// 如果存在预构建的渲染器则使用它，否则从输入路径构建。
    /// 对于字节输入且无预构建渲染器的情况，写入临时文件。
    fn get_renderer<'a>(
        &'a self,
        input: &PdfInput,
    ) -> Result<Box<dyn crate::render::PdfRenderer + 'a>> {
        if let Some(ref renderer) = self.renderer {
            // 已注入预构建渲染器（例如用于测试）。
            // 由于无法克隆 trait 对象，将其包装在委托适配器中。
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

    /// 根据触发策略判断页面是否需要 OCR。
    fn page_needs_ocr(&self, page: &PdfPageModel) -> bool {
        match self.config.trigger {
            OcrTrigger::Always => true,
            OcrTrigger::OnEmptyPage => {
                // 如果页面没有段落或标题块则执行 OCR。
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
                // 阈值比较：text_count / total < threshold。
                // 改写为整数运算：text_count * 1000 < total * (threshold * 1000)。
                // 页面计数始终很小，乘法不会溢出。
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let threshold_permille = (f64::from(threshold) * 1000.0).round() as usize;
                text_count.saturating_mul(1000) < total.saturating_mul(threshold_permille)
            }
        }
    }

    /// 渲染页面并对其执行 OCR。
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

        // 按最小文本长度过滤。
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
                        // OCR 未返回可用文本。
                        warnings.push(MarkdownWarning::OcrUnavailable {
                            page_index: page.index(),
                        });
                        new_pages.push(page.clone());
                        continue;
                    }

                    // 检查置信度阈值。
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

                    // 构建注入 OCR 文本后的新页面。
                    let mut new_page = PdfPageModel::new(page.index());
                    if let (Some(w), Some(h)) = (page.width_pt(), page.height_pt()) {
                        new_page = new_page.with_dimensions(w, h);
                    }
                    new_page = new_page.with_rotation(page.rotation());

                    // 保留现有块。
                    for block in page.blocks() {
                        new_page = new_page.with_block(block.clone());
                    }

                    // 将 OCR 文本作为新段落注入。
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
