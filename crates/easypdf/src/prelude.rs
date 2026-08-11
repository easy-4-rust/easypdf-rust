//! Convenience re-exports for `easypdf` usage.

pub use super::EasyPdf;

// Core types
pub use easypdf_core::*;
pub use easypdf_derive::PdfModel;

// Model types
pub use easypdf_core::{
    ImageData, ImageFormat, ListItem, PdfBlock, PdfBlockType, PdfDocumentModel, PdfPageModel,
    SourceLocation,
};

// I/O types
pub use easypdf_core::{AtomicFileOutput, PdfInput, ResourceLimits};

// Reader / Writer
pub use easypdf_reader::{PdfReader, ReadStrategy};
pub use easypdf_writer::{PdfWriter, PdfWriterBuilder, WriteBackend};

// Manipulate / Template
pub use easypdf_reader::PdfManipulator;
pub use easypdf_writer::PdfTemplateFiller;

// Layout
pub use easypdf_core::layout::Direction as LayoutDirection;
pub use easypdf_core::layout::FlowLayout;

// Markdown (optional)
#[cfg(feature = "markdown")]
pub use easypdf_markdown::{
    ImagePolicy, MarkdownProfile, OcrPolicy, PdfMarkdownBuilder, PdfMarkdownProcessor,
    ProcessorPipeline, TablePolicy,
};

// Table detection (optional)
#[cfg(feature = "markdown-table")]
pub use easypdf_markdown::table::{TableDetectionConfig, TableDetectorProcessor};

// OCR (optional)
#[cfg(feature = "ocr")]
pub use easypdf_markdown::ocr::{OcrConfig, OcrEngine, OcrProcessor, OcrResult, OcrTrigger};

// Render (optional)
#[cfg(feature = "render")]
pub use easypdf_markdown::render::{PdfRenderer, RenderBackend, RenderConfig};

// Resident (optional)
#[cfg(feature = "resident")]
pub use easypdf_runtime::resident::{AutosaveMode, ResidentClient, ResidentServer};

// MCP (optional)
#[cfg(feature = "mcp")]
pub use easypdf_runtime::mcp::McpServer;
