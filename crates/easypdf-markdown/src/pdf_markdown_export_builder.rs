//! PDF 到 Markdown 的易用构建器。

use std::ops::Range;
use std::path::{Path, PathBuf};

use easypdf_core::Result;
use easypdf_io::{AtomicFileOutput, PdfInput, ResourceLimits};
use easypdf_reader::PdfReader;

use crate::{
    ImagePolicy, MarkdownExportReport, MarkdownExportResult, MarkdownProfile, MarkdownRenderer,
    MarkdownWarning, OcrPolicy, TablePolicy,
};

/// PDF 到 Markdown 的链式导出构建器。
#[derive(Clone, Debug)]
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

    /// 执行转换并原子写入输出文件。
    ///
    /// # Errors
    ///
    /// 输入读取、PDF 解析、文本提取或输出写入失败时返回错误。
    pub fn do_export(self) -> Result<MarkdownExportResult> {
        let image_extraction_requested = matches!(&self.image_policy, ImagePolicy::ExtractTo(_));
        let mut reader = PdfReader::open_with_limits(&self.input, self.limits)?;
        if let Some(pages) = self.pages {
            reader = reader.try_pages(pages)?;
        }
        let document = reader.extract_document_model()?;
        let renderer = MarkdownRenderer::new(self.profile)
            .with_table_policy(self.table_policy)
            .with_image_policy(self.image_policy);
        let markdown = renderer.render(&document);
        AtomicFileOutput::new(&self.output).write(markdown.as_bytes())?;

        let mut warnings = Vec::new();
        if self.table_policy == TablePolicy::Detect {
            warnings.push(MarkdownWarning::TableDetectionUnavailable);
        }
        if image_extraction_requested {
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
            if self.ocr_policy == OcrPolicy::Auto {
                warnings.push(MarkdownWarning::OcrUnavailable {
                    page_index: page.index(),
                });
            }
        }
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
        Ok(MarkdownExportResult::new(self.output, report))
    }

    /// 返回输出目标路径。
    #[must_use]
    pub fn output(&self) -> &Path {
        &self.output
    }
}
