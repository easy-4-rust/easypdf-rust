//! # easypdf-rust
//!
//! 符合 Rust 惯用法的 PDF 操作库：创建、读取、操作和表单填充。
//! 受阿里巴巴 EasyExcel 的 builder 模式 API 设计启发。
//!
//! ## 快速示例
//!
//! **创建 PDF：**
//! ```ignore
//! use easypdf::prelude::*;
//!
//! EasyPdf::create("output.pdf")
//!     .page(PageSize::A4)
//!     .add_text("Hello, world!")
//!         .font(PdfFont::helvetica(12.0))
//!     .do_write()?;
//! ```
//!
//! **读取 PDF：**
//! ```ignore
//! let text = EasyPdf::read("input.pdf").extract_text()?;
//! ```
//!
//! **合并 PDF：**
//! ```ignore
//! EasyPdf::merge(&["a.pdf", "b.pdf"], "merged.pdf")?;
//! ```
//!
//! **填充表单：**
//! ```ignore
//! #[derive(PdfModel)]
//! struct MyData {
//!     #[pdf(field = "name")]
//!     name: String,
//! }
//!
//! EasyPdf::fill_form("template.pdf", &MyData { name: "Alice".into() })
//!     .save("filled.pdf")?;
//! ```

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![deny(unsafe_code)]
#![allow(
    clippy::uninlined_format_args,
    clippy::manual_string_new,
    clippy::cast_precision_loss,
    clippy::doc_markdown,
    clippy::write_with_newline,
    clippy::items_after_statements
)]

// ======================================================================
// 来自子 crate 的重导出
// ======================================================================

// --- 核心类型（平坦重导出） ---
pub use easypdf_core::*;

// --- derive 宏 ---
pub use easypdf_derive::PdfModel;

// --- 读取器 ---
pub use easypdf_reader::{PdfReader, ReadStrategy};

// --- 引擎中立模型与受限 I/O ---
pub use easypdf_core::io::guards::{guard_decompression_bomb, guard_element_explosion};
pub use easypdf_core::io::repair::{RepairOptions, attempt_repair, is_likely_corrupt};
pub use easypdf_core::io::ssrf_guard::validate_url as validate_io_url;
pub use easypdf_core::{AtomicFileOutput, PdfInput, ResourceLimits};
pub use easypdf_core::{
    ImageData, ImageFormat, ListItem, PdfBlock, PdfBlockType, PdfDocumentModel, PdfPageModel,
    SourceLocation,
};

// --- 写入器 ---
pub use easypdf_writer::{PdfWriter, PdfWriterBuilder, WriteBackend, WriteEngineKind};

// --- 操作器 ---
pub use easypdf_reader::PdfManipulator;

// --- 模板填充 ---
pub use easypdf_writer::PdfTemplateFiller;

// --- 布局 crate ---
pub use easypdf_core::layout::Direction as LayoutDirection;
pub use easypdf_core::layout::FlowLayout;

// --- Markdown 管道（可选） ---
#[cfg(feature = "markdown")]
pub use easypdf_markdown::{
    DetailedProcessorCapabilities, PRIORITY_GENERIC, PRIORITY_SPECIFIC, ProcessorCapability,
    ProcessorPipeline,
};
#[cfg(feature = "markdown")]
pub use easypdf_markdown::{
    ImagePolicy, MarkdownConversionResult, MarkdownExportReport, MarkdownExportResult,
    MarkdownProcessorCapabilities, MarkdownProfile, MarkdownRenderer, MarkdownWarning, OcrPolicy,
    PdfMarkdownBuilder, PdfMarkdownExportBuilder, PdfMarkdownProcessor, TablePolicy,
};

// --- 表格检测（可选） ---
#[cfg(feature = "markdown-table")]
pub use easypdf_markdown::table::{ColumnSeparator, TableDetectionConfig, TableDetectorProcessor};

// --- OCR 管道（可选） ---
#[cfg(feature = "ocr")]
pub use easypdf_markdown::ocr::{
    OcrConfig, OcrEngine, OcrImage, OcrProcessor, OcrResult, OcrTrigger, WordBox,
};

// --- OCR 引擎（可选） ---
#[cfg(feature = "ocr")]
pub use easypdf_ocr::{
    AuthMethod as OcrAuthMethod, BackoffStrategy, HttpClientConfig as OcrHttpClientConfig,
    HttpOcrEngine, OcrHttpError, RateLimitConfig,
};

// --- 渲染（可选） ---
#[cfg(feature = "render")]
pub use easypdf_markdown::render::error::RenderError;
#[cfg(feature = "render")]
pub use easypdf_markdown::render::{
    Background, PdfRenderer, RenderBackend, RenderConfig, RenderedImage,
};

// --- Resident 守护进程（可选） ---
#[cfg(feature = "resident")]
pub use easypdf_runtime::resident::serve as resident_serve;
#[cfg(feature = "resident")]
pub use easypdf_runtime::resident::try_attach as resident_try_attach;
#[cfg(feature = "resident")]
pub use easypdf_runtime::resident::{AutosaveMode, ResidentClient, ResidentConfig, ResidentServer};

// --- MCP 服务器（可选） ---
#[cfg(feature = "mcp")]
pub use easypdf_runtime::mcp::McpServer;

// ======================================================================
// 内部模块
// ======================================================================

mod builders;
pub use builders::{
    PdfCreateBuilder, PdfImageBuilder, PdfManipulateBuilder, PdfPositionedTextBuilder,
    PdfReadBuilder, PdfSplitBuilder, PdfTableBuilder, PdfTextBuilder,
};

mod pdf_fill_builder;
pub use pdf_fill_builder::PdfFillBuilder;

mod writer_helpers;
pub use writer_helpers::{PageNumberHandler, write_table};

pub mod prelude;

#[cfg(any(test, feature = "html"))]
mod html;
#[cfg(feature = "html")]
pub use html::HtmlToPdfBuilder;
#[cfg(test)]
use html::markdown_to_html;
mod crypto_facade;
mod facade_features;

mod easy_pdf;
pub use easy_pdf::EasyPdf;

#[cfg(test)]
mod tests;
