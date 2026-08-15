//! 将 PDF 语义内容确定性转换为 Markdown。
//!
//! # 架构
//!
//! ```text
//! PdfInput → PdfReader → PdfDocumentModel → ProcessorPipeline → MarkdownRenderer → String
//! ```
//!
//! - [`PdfMarkdownProcessor`] trait 定义单个语义增强处理器
//! - [`ProcessorPipeline`] 按优先级组合多个处理器
//! - [`MarkdownRenderer`] 将模型渲染为 Markdown 字符串
//! - [`MarkdownProfile`] 及 [`MarkdownProfileBuilder`] 配置输出格式与管道
//!
//! # 快速开始
//!
//! ```no_run
//! use easypdf_markdown::{
//!     MarkdownProfile, ProcessorPipeline, PdfMarkdownBuilder,
//!     processors::{ReadingOrderProcessor, HeadingDetectorProcessor},
//! };
//!
//! // 构建处理器管道
//! let mut pipeline = ProcessorPipeline::new();
//! pipeline.register(Box::new(ReadingOrderProcessor));
//! pipeline.register(Box::new(HeadingDetectorProcessor::new()));
//!
//! // 或使用 Profile 预设
//! let config = MarkdownProfile::balanced();
//! ```

#![deny(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::pedantic)]

mod image_policy;
mod markdown_conversion_result;
mod markdown_export_report;
mod markdown_export_result;
mod markdown_processor_capabilities;
mod markdown_profile;
mod markdown_renderer;
mod markdown_warning;
mod ocr_policy;
mod pdf_markdown_builder;
mod pdf_markdown_export_builder;
mod pdf_markdown_processor;
mod processor_capability;
mod processor_pipeline;
mod table_policy;

/// 内置开箱即用的语义处理器。
pub mod processors;

/// PDF page rendering to raster images (merged from `easypdf-render`).
pub mod render;

/// Heuristic table detection (merged from `easypdf-markdown-table`).
pub mod table;

/// OCR processor for scanned PDF text extraction (merged from `easypdf-markdown-ocr`).
pub mod ocr;

pub use image_policy::ImagePolicy;
pub use markdown_conversion_result::MarkdownConversionResult;
pub use markdown_export_report::MarkdownExportReport;
pub use markdown_export_result::MarkdownExportResult;
pub use markdown_processor_capabilities::MarkdownProcessorCapabilities;
pub use markdown_profile::{BuildPolicies, MarkdownProfile, MarkdownProfileBuilder};
pub use markdown_renderer::MarkdownRenderer;
pub use markdown_warning::MarkdownWarning;
pub use ocr_policy::OcrPolicy;
pub use pdf_markdown_builder::PdfMarkdownBuilder;
pub use pdf_markdown_export_builder::PdfMarkdownExportBuilder;
pub use pdf_markdown_processor::PdfMarkdownProcessor;
pub use processor_capability::{DetailedProcessorCapabilities, ProcessorCapability};
pub use processor_pipeline::{PRIORITY_GENERIC, PRIORITY_SPECIFIC, ProcessorPipeline};
pub use table_policy::TablePolicy;

// --- Flat re-exports from render submodule (backward compatibility) ---
pub use render::{
    Background, ImageFormat, PdfRenderer, RenderBackend, RenderConfig, RenderError, RenderedImage,
};

// --- Flat re-exports from table submodule (backward compatibility) ---
pub use table::{ColumnSeparator, TableDetectionConfig, TableDetectorProcessor};

// --- Flat re-exports from ocr submodule (backward compatibility) ---
pub use ocr::{OcrConfig, OcrEngine, OcrImage, OcrProcessor, OcrResult, OcrTrigger, WordBox};
