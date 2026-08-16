//! `easypdf` 的便捷重导出。

pub use super::EasyPdf;

// 核心类型
pub use easypdf_core::*;
pub use easypdf_derive::PdfModel;

// 模型类型
pub use easypdf_core::{
    ImageData, ImageFormat, ListItem, PdfBlock, PdfBlockType, PdfDocumentModel, PdfPageModel,
    SourceLocation,
};

// I/O 类型
pub use easypdf_core::{AtomicFileOutput, PdfInput, ResourceLimits};

// 读取器 / 写入器
pub use easypdf_reader::{PdfReader, ReadStrategy};
pub use easypdf_writer::{PdfWriter, PdfWriterBuilder, WriteBackend, WriteEngineKind};

// 操作器 / 模板
pub use easypdf_reader::PdfManipulator;
pub use easypdf_writer::PdfTemplateFiller;

// 布局
pub use easypdf_core::layout::Direction as LayoutDirection;
pub use easypdf_core::layout::FlowLayout;

// Markdown（可选）
#[cfg(feature = "markdown")]
pub use easypdf_markdown::{
    ImagePolicy, MarkdownProfile, OcrPolicy, PdfMarkdownBuilder, PdfMarkdownProcessor,
    ProcessorPipeline, TablePolicy,
};

// 表格检测（可选）
#[cfg(feature = "markdown-table")]
pub use easypdf_markdown::table::{TableDetectionConfig, TableDetectorProcessor};

// OCR（可选）
#[cfg(feature = "ocr")]
pub use easypdf_markdown::ocr::{OcrConfig, OcrEngine, OcrProcessor, OcrResult, OcrTrigger};

// 渲染（可选）
#[cfg(feature = "render")]
pub use easypdf_markdown::render::{PdfRenderer, RenderBackend, RenderConfig};

// Resident（可选）
#[cfg(feature = "resident")]
pub use easypdf_runtime::resident::{AutosaveMode, ResidentClient, ResidentServer};

// MCP（可选）
#[cfg(feature = "mcp")]
pub use easypdf_runtime::mcp::McpServer;
