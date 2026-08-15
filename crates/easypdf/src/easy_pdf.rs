//! EasyPdf 门面类型 -- 所有 easypdf-rust 操作的统一入口。
//!
//! 提供静态工厂方法，返回符合人体工学的 builder 链，
//! 用于创建、读取、操作和填充 PDF 文档。

use std::path::{Path, PathBuf};

use crate::builders::{PdfCreateBuilder, PdfManipulateBuilder, PdfReadBuilder, PdfSplitBuilder};
use crate::pdf_fill_builder::PdfFillBuilder;

#[cfg(feature = "html")]
use crate::html::{HtmlToPdfBuilder, markdown_to_html};

/// `easypdf-rust` 所有操作的主入口。
///
/// 提供静态工厂方法，返回符合人体工学的 builder 链，
/// 用于创建、读取、操作和填充 PDF 文档。
///
/// # 示例
///
/// ```ignore
/// use easypdf::EasyPdf;
///
/// // 创建 PDF
/// EasyPdf::create("output.pdf")
///     .page_size(PageSize::A4)
///     .add_text("Hello, world!")
///     .font(PdfFont::helvetica(12.0))
///     .do_write()?;
///
/// // 读取 PDF
/// let text = EasyPdf::read("input.pdf").extract_text()?;
/// ```
pub struct EasyPdf;

impl EasyPdf {
    // --- 创建 ---

    /// 开始构建新的 PDF 文档。
    ///
    /// 返回一个 [`PdfCreateBuilder`]，用于配置页面、内容和元数据。
    ///
    /// # 参数
    ///
    /// * `path` - 输出 PDF 文件路径，可接受任何能转换为 [`PathBuf`] 的类型。
    #[must_use = "builder method"]
    pub fn create(path: impl Into<PathBuf>) -> PdfCreateBuilder {
        PdfCreateBuilder::new(path)
    }

    // --- HTML / Markdown ---

    /// 从 HTML 字符串创建 PDF（需要 `html` feature 和 Chromium）。
    ///
    /// # 参数
    ///
    /// * `html` - 要渲染为 PDF 的 HTML 内容。
    ///
    /// # Errors
    ///
    /// 如果 Chromium 不可用或 HTML 无法渲染，返回错误。
    #[cfg(feature = "html")]
    pub fn from_html(html: &str) -> crate::Result<HtmlToPdfBuilder> {
        Ok(HtmlToPdfBuilder::new(html))
    }

    /// 从 Markdown 字符串创建 PDF（需要 `html` feature 和 Chromium）。
    ///
    /// 分两步转换：Markdown -> HTML -> PDF。
    ///
    /// # 参数
    ///
    /// * `md` - 要渲染为 PDF 的 Markdown 内容。
    ///
    /// # Errors
    ///
    /// 如果 Chromium 不可用或 Markdown 无法渲染，返回错误。
    #[cfg(feature = "html")]
    pub fn from_markdown(md: &str) -> crate::Result<HtmlToPdfBuilder> {
        let html = markdown_to_html(md);
        Ok(HtmlToPdfBuilder::new(&html))
    }

    // --- 读取 ---

    /// 开始构建 PDF 文本提取读取器。
    ///
    /// 返回一个 [`PdfReadBuilder`]，用于配置页面范围和提取模式。
    ///
    /// # 参数
    ///
    /// * `path` - 要读取的 PDF 文件路径。
    #[must_use = "builder method"]
    pub fn read(path: impl Into<PathBuf>) -> PdfReadBuilder {
        PdfReadBuilder::new(path)
    }

    /// 开始 PDF 转 Markdown 导出操作。
    ///
    /// 导出器仅解析 PDF 一次，应用资源限制，并在转换成功后
    /// 原子性地替换输出文件。
    ///
    /// # 参数
    ///
    /// * `input` - 输入 PDF 文件路径。
    /// * `output` - 输出 Markdown 文件路径。
    #[cfg(feature = "markdown")]
    #[must_use = "builder method"]
    pub fn export_markdown(
        input: impl Into<PathBuf>,
        output: impl Into<PathBuf>,
    ) -> crate::PdfMarkdownExportBuilder {
        crate::PdfMarkdownExportBuilder::new(input, output)
    }

    /// 开始 PDF 转 Markdown 内存转换操作。
    ///
    /// 返回的 builder 解析 PDF 一次，同时返回 Markdown 文本和
    /// 结构化转换报告，无需临时输出文件。
    ///
    /// # 参数
    ///
    /// * `input` - 输入 PDF 文件路径。
    #[cfg(feature = "markdown")]
    #[must_use = "builder method"]
    pub fn to_markdown(input: impl Into<PathBuf>) -> crate::PdfMarkdownBuilder {
        crate::PdfMarkdownBuilder::new(input)
    }

    // --- 合并 ---

    /// 将多个 PDF 文件合并为一个输出文件。
    ///
    /// # 参数
    ///
    /// * `input_paths` - 输入 PDF 文件路径列表。
    /// * `output` - 合并后的输出文件路径。
    ///
    /// # Errors
    ///
    /// 如果任何输入文件无法读取或输出无法写入，返回错误。
    pub fn merge(input_paths: &[impl AsRef<Path>], output: impl AsRef<Path>) -> crate::Result<()> {
        easypdf_reader::PdfManipulator::merge_files(input_paths, output)
    }

    // --- 拆分 ---

    /// 开始构建 PDF 拆分操作。
    ///
    /// # 参数
    ///
    /// * `path` - 要拆分的 PDF 文件路径。
    #[must_use = "builder method"]
    pub fn split(path: impl Into<PathBuf>) -> PdfSplitBuilder {
        PdfSplitBuilder::new(path)
    }

    // --- 操作 ---

    /// 开始构建 PDF 操作（旋转、重排等）。
    ///
    /// # 参数
    ///
    /// * `path` - 要操作的 PDF 文件路径。
    #[must_use = "builder method"]
    pub fn manipulate(path: impl Into<PathBuf>) -> PdfManipulateBuilder {
        PdfManipulateBuilder::new(path)
    }

    // --- 表单填充 ---

    /// 使用数据填充 PDF 表单模板。
    ///
    /// 返回一个 [`PdfFillBuilder`]，用于配置字段值并保存。
    ///
    /// # 参数
    ///
    /// * `template_path` - PDF 表单模板文件路径。
    /// * `data` - 实现 [`easypdf_core::PdfModel`] trait 的数据对象。
    #[must_use = "builder method"]
    pub fn fill_form(
        template_path: impl Into<PathBuf>,
        data: &dyn easypdf_core::PdfModel,
    ) -> PdfFillBuilder {
        PdfFillBuilder::new(template_path, data)
    }
}
