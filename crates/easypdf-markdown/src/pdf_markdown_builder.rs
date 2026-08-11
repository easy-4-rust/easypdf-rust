//! PDF 到 Markdown 的内存转换构建器。

use std::ops::Range;
use std::sync::Arc;

use easypdf_core::Result;
use easypdf_core::{PdfInput, ResourceLimits};
use easypdf_reader::PdfReader;

use crate::{
    ImagePolicy, MarkdownConversionResult, MarkdownExportReport, MarkdownProcessorCapabilities,
    MarkdownProfile, MarkdownRenderer, MarkdownWarning, OcrPolicy, PdfMarkdownProcessor,
    TablePolicy,
};

/// PDF 到 Markdown 的链式内存转换构建器。
#[derive(Clone)]
#[must_use]
pub struct PdfMarkdownBuilder {
    input: PdfInput,
    pages: Option<Range<usize>>,
    profile: MarkdownProfile,
    table_policy: TablePolicy,
    image_policy: ImagePolicy,
    ocr_policy: OcrPolicy,
    limits: ResourceLimits,
    processors: Vec<Arc<dyn PdfMarkdownProcessor>>,
}

impl std::fmt::Debug for PdfMarkdownBuilder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PdfMarkdownBuilder")
            .field("input", &self.input)
            .field("pages", &self.pages)
            .field("profile", &self.profile)
            .field("table_policy", &self.table_policy)
            .field("image_policy", &self.image_policy)
            .field("ocr_policy", &self.ocr_policy)
            .field("limits", &self.limits)
            .field("processor_count", &self.processors.len())
            .finish()
    }
}

impl PdfMarkdownBuilder {
    /// 从路径创建转换任务。
    pub fn new(input: impl Into<std::path::PathBuf>) -> Self {
        Self::from_input(PdfInput::from_path(input.into()))
    }

    /// 从内存 PDF 字节创建转换任务。
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Self {
        Self::from_input(PdfInput::from_bytes(bytes))
    }

    /// 从统一 PDF 输入创建转换任务。
    #[must_use = "conversion builder must be configured or executed"]
    pub fn from_input(input: PdfInput) -> Self {
        Self {
            input,
            pages: None,
            profile: MarkdownProfile::default(),
            table_policy: TablePolicy::default(),
            image_policy: ImagePolicy::default(),
            ocr_policy: OcrPolicy::default(),
            limits: ResourceLimits::default(),
            processors: Vec::new(),
        }
    }

    /// 设置零基页范围。
    pub fn pages(mut self, pages: Range<usize>) -> Self {
        self.pages = Some(pages);
        self
    }

    /// 设置 Markdown 输出配置档。
    pub const fn profile(mut self, profile: MarkdownProfile) -> Self {
        self.profile = profile;
        self
    }

    /// 设置表格策略。
    pub const fn tables(mut self, policy: TablePolicy) -> Self {
        self.table_policy = policy;
        self
    }

    /// 设置图片策略。
    pub fn images(mut self, policy: ImagePolicy) -> Self {
        self.image_policy = policy;
        self
    }

    /// 设置 OCR 回退策略。
    pub const fn ocr(mut self, policy: OcrPolicy) -> Self {
        self.ocr_policy = policy;
        self
    }

    /// 设置资源限制。
    pub const fn resource_limits(mut self, limits: ResourceLimits) -> Self {
        self.limits = limits;
        self
    }

    /// 注册一个语义增强处理器。
    pub fn processor(mut self, processor: Arc<dyn PdfMarkdownProcessor>) -> Self {
        self.processors.push(processor);
        self
    }

    /// 解析一次 PDF，并在内存中返回 Markdown 和结构化报告。
    ///
    /// # Errors
    ///
    /// 输入读取、PDF 解析、文本提取或任一处理器失败时返回错误。
    pub fn do_convert(self) -> Result<MarkdownConversionResult> {
        let mut reader = PdfReader::open_with_limits(&self.input, self.limits)?;
        if let Some(pages) = self.pages {
            reader = reader.try_pages(pages)?;
        }
        let mut document = reader.extract_document_model()?;
        let mut warnings = Vec::new();
        let mut capabilities = MarkdownProcessorCapabilities::new();
        for processor in &self.processors {
            capabilities = capabilities.union(processor.capabilities());
            let (processed, mut processor_warnings) = processor.process(&self.input, document)?;
            document = processed;
            warnings.append(&mut processor_warnings);
        }

        append_capability_warnings(
            &document,
            self.table_policy,
            &self.image_policy,
            self.ocr_policy,
            capabilities,
            &mut warnings,
        );
        let renderer = MarkdownRenderer::new(self.profile)
            .with_table_policy(self.table_policy)
            .with_image_policy(self.image_policy);
        let markdown = renderer.render(&document);
        let blocks_written = document
            .pages()
            .iter()
            .map(|page| page.blocks().len())
            .sum();
        let report = MarkdownExportReport::new(
            document.page_count(),
            blocks_written,
            markdown.len(),
            warnings,
        );
        Ok(MarkdownConversionResult::new(markdown, report))
    }
}

fn append_capability_warnings(
    document: &easypdf_core::PdfDocumentModel,
    table_policy: TablePolicy,
    image_policy: &ImagePolicy,
    ocr_policy: OcrPolicy,
    capabilities: MarkdownProcessorCapabilities,
    warnings: &mut Vec<MarkdownWarning>,
) {
    if table_policy == TablePolicy::Detect && !capabilities.table_detection() {
        warnings.push(MarkdownWarning::TableDetectionUnavailable);
    }
    if matches!(image_policy, ImagePolicy::ExtractTo(_)) && !capabilities.image_extraction() {
        warnings.push(MarkdownWarning::ImageExtractionUnavailable);
    }
    for page in document
        .pages()
        .iter()
        .filter(|page| page.blocks().is_empty())
    {
        warnings.push(MarkdownWarning::EmptyPage {
            page_index: page.index(),
        });
        if ocr_policy == OcrPolicy::Auto && !capabilities.ocr() {
            warnings.push(MarkdownWarning::OcrUnavailable {
                page_index: page.index(),
            });
        }
    }
}

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_builder() {
        let builder = PdfMarkdownBuilder::new("/tmp/test.pdf");
        assert!(builder.pages.is_none());
        assert_eq!(builder.profile, MarkdownProfile::default());
    }

    #[test]
    fn from_bytes_creates_builder() {
        let builder = PdfMarkdownBuilder::from_bytes(vec![1, 2, 3]);
        assert!(builder.pages.is_none());
    }

    #[test]
    fn from_input_creates_builder() {
        let input = PdfInput::from_bytes(vec![1, 2]);
        let builder = PdfMarkdownBuilder::from_input(input);
        assert!(builder.pages.is_none());
    }

    #[test]
    fn pages_sets_range() {
        let builder = PdfMarkdownBuilder::new("/tmp/test.pdf").pages(1..5);
        assert_eq!(builder.pages, Some(1..5));
    }

    #[test]
    fn profile_sets_profile() {
        let builder = PdfMarkdownBuilder::new("/tmp/test.pdf").profile(MarkdownProfile::Llm);
        assert_eq!(builder.profile, MarkdownProfile::Llm);
    }

    #[test]
    fn tables_sets_policy() {
        let builder = PdfMarkdownBuilder::new("/tmp/test.pdf").tables(TablePolicy::PlainText);
        assert_eq!(builder.table_policy, TablePolicy::PlainText);
    }

    #[test]
    fn images_sets_policy() {
        let builder = PdfMarkdownBuilder::new("/tmp/test.pdf").images(ImagePolicy::Reference);
        assert_eq!(builder.image_policy, ImagePolicy::Reference);
    }

    #[test]
    fn ocr_sets_policy() {
        let builder = PdfMarkdownBuilder::new("/tmp/test.pdf").ocr(OcrPolicy::Auto);
        assert_eq!(builder.ocr_policy, OcrPolicy::Auto);
    }

    #[test]
    fn resource_limits_sets_limits() {
        let limits = ResourceLimits::strict();
        let builder = PdfMarkdownBuilder::new("/tmp/test.pdf").resource_limits(limits);
        assert_eq!(builder.limits.max_input_bytes(), limits.max_input_bytes());
    }

    #[test]
    fn debug_format() {
        let builder = PdfMarkdownBuilder::new("/tmp/test.pdf");
        let dbg = format!("{:?}", builder);
        assert!(dbg.contains("PdfMarkdownBuilder"));
    }

    #[test]
    fn clone_preserves_values() {
        let builder = PdfMarkdownBuilder::new("/tmp/test.pdf")
            .profile(MarkdownProfile::Llm)
            .tables(TablePolicy::PlainText);
        let cloned = builder.clone();
        assert_eq!(builder.profile, cloned.profile);
        assert_eq!(builder.table_policy, cloned.table_policy);
    }

    #[test]
    fn chaining_sets_all_fields() {
        let builder = PdfMarkdownBuilder::new("/tmp/test.pdf")
            .pages(0..10)
            .profile(MarkdownProfile::Gfm)
            .tables(TablePolicy::Detect)
            .images(ImagePolicy::Ignore)
            .ocr(OcrPolicy::Disabled)
            .resource_limits(ResourceLimits::permissive());
        assert_eq!(builder.pages, Some(0..10));
        assert_eq!(builder.profile, MarkdownProfile::Gfm);
        assert_eq!(builder.table_policy, TablePolicy::Detect);
        assert_eq!(builder.image_policy, ImagePolicy::Ignore);
        assert_eq!(builder.ocr_policy, OcrPolicy::Disabled);
    }
}
