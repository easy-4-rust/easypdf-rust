//! PDF 到 Markdown 的易用构建器。

use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use easypdf_core::Result;
use easypdf_core::{AtomicFileOutput, PdfInput, ResourceLimits};

use crate::{
    ImagePolicy, MarkdownExportResult, MarkdownProfile, OcrPolicy, PdfMarkdownBuilder,
    PdfMarkdownProcessor, TablePolicy,
};

/// PDF 到 Markdown 的链式导出构建器。
#[derive(Clone)]
#[must_use]
pub struct PdfMarkdownExportBuilder {
    input: PdfInput,
    output: PathBuf,
    pages: Option<Range<usize>>,
    profile: MarkdownProfile,
    table_policy: TablePolicy,
    image_policy: ImagePolicy,
    ocr_policy: OcrPolicy,
    limits: ResourceLimits,
    processors: Vec<Arc<dyn PdfMarkdownProcessor>>,
}

impl std::fmt::Debug for PdfMarkdownExportBuilder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PdfMarkdownExportBuilder")
            .field("input", &self.input)
            .field("output", &self.output)
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

impl PdfMarkdownExportBuilder {
    /// 创建 PDF 到 Markdown 导出任务。
    pub fn new(input: impl Into<PathBuf>, output: impl Into<PathBuf>) -> Self {
        Self {
            input: PdfInput::from_path(input.into()),
            output: output.into(),
            pages: None,
            profile: MarkdownProfile::default(),
            table_policy: TablePolicy::default(),
            image_policy: ImagePolicy::default(),
            ocr_policy: OcrPolicy::default(),
            limits: ResourceLimits::default(),
            processors: Vec::new(),
        }
    }

    /// 从内存 PDF 字节创建导出任务。
    pub fn from_bytes(bytes: impl Into<Vec<u8>>, output: impl Into<PathBuf>) -> Self {
        Self {
            input: PdfInput::from_bytes(bytes),
            output: output.into(),
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

    /// 执行转换并原子写入输出文件。
    ///
    /// # Errors
    ///
    /// 输入读取、PDF 解析、文本提取或输出写入失败时返回错误。
    pub fn do_export(self) -> Result<MarkdownExportResult> {
        let mut builder = PdfMarkdownBuilder::from_input(self.input)
            .profile(self.profile)
            .tables(self.table_policy)
            .images(self.image_policy)
            .ocr(self.ocr_policy)
            .resource_limits(self.limits);
        if let Some(pages) = self.pages {
            builder = builder.pages(pages);
        }
        for processor in self.processors {
            builder = builder.processor(processor);
        }
        let conversion = builder.do_convert()?;
        AtomicFileOutput::new(&self.output).write(conversion.markdown().as_bytes())?;
        Ok(MarkdownExportResult::new(
            self.output,
            conversion.report().clone(),
        ))
    }

    /// 返回输出目标路径。
    #[must_use]
    pub fn output(&self) -> &Path {
        &self.output
    }
}

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_builder() {
        let builder = PdfMarkdownExportBuilder::new("/tmp/input.pdf", "/tmp/output.md");
        assert_eq!(builder.output(), Path::new("/tmp/output.md"));
    }

    #[test]
    fn from_bytes_creates_builder() {
        let builder = PdfMarkdownExportBuilder::from_bytes(vec![1, 2, 3], "/tmp/out.md");
        assert_eq!(builder.output(), Path::new("/tmp/out.md"));
    }

    #[test]
    fn pages_sets_range() {
        let builder = PdfMarkdownExportBuilder::new("/tmp/in.pdf", "/tmp/out.md").pages(2..5);
        assert!(builder.pages.is_some());
        assert_eq!(builder.pages.unwrap(), 2..5);
    }

    #[test]
    fn profile_sets_profile() {
        let builder = PdfMarkdownExportBuilder::new("/tmp/in.pdf", "/tmp/out.md")
            .profile(MarkdownProfile::Llm);
        assert_eq!(builder.profile, MarkdownProfile::Llm);
    }

    #[test]
    fn tables_sets_policy() {
        let builder = PdfMarkdownExportBuilder::new("/tmp/in.pdf", "/tmp/out.md")
            .tables(TablePolicy::PlainText);
        assert_eq!(builder.table_policy, TablePolicy::PlainText);
    }

    #[test]
    fn images_sets_policy() {
        let builder = PdfMarkdownExportBuilder::new("/tmp/in.pdf", "/tmp/out.md")
            .images(ImagePolicy::Reference);
        assert_eq!(builder.image_policy, ImagePolicy::Reference);
    }

    #[test]
    fn ocr_sets_policy() {
        let builder =
            PdfMarkdownExportBuilder::new("/tmp/in.pdf", "/tmp/out.md").ocr(OcrPolicy::Auto);
        assert_eq!(builder.ocr_policy, OcrPolicy::Auto);
    }

    #[test]
    fn resource_limits_sets_limits() {
        let limits = ResourceLimits::strict();
        let builder =
            PdfMarkdownExportBuilder::new("/tmp/in.pdf", "/tmp/out.md").resource_limits(limits);
        assert_eq!(builder.limits.max_input_bytes(), limits.max_input_bytes());
    }

    #[test]
    fn debug_format() {
        let builder = PdfMarkdownExportBuilder::new("/tmp/in.pdf", "/tmp/out.md");
        let dbg = format!("{:?}", builder);
        assert!(dbg.contains("PdfMarkdownExportBuilder"));
        assert!(dbg.contains("input"));
        assert!(dbg.contains("output"));
    }

    #[test]
    fn clone_preserves_values() {
        let builder = PdfMarkdownExportBuilder::new("/tmp/in.pdf", "/tmp/out.md")
            .profile(MarkdownProfile::Llm);
        let cloned = builder.clone();
        assert_eq!(builder.profile, cloned.profile);
        assert_eq!(builder.output(), cloned.output());
    }
}
