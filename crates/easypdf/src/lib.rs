//! # easypdf-rust
//!
//! An idiomatic Rust library for quick PDF operations: creation, reading,
//! manipulation, and template filling. Inspired by Alibaba EasyExcel's
//! builder-pattern API design.
//!
//! ## Quick examples
//!
//! **Create a PDF:**
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
//! **Read a PDF:**
//! ```ignore
//! let text = EasyPdf::read("input.pdf").extract_text()?;
//! ```
//!
//! **Merge PDFs:**
//! ```ignore
//! EasyPdf::merge(&["a.pdf", "b.pdf"], "merged.pdf")?;
//! ```
//!
//! **Fill a form:**
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
// Re-exports from sub-crates
// ======================================================================

// --- Core types (flat re-export) ---
pub use easypdf_core::*;

// --- Derive macro ---
pub use easypdf_derive::PdfModel;

// --- Reader ---
pub use easypdf_reader::{PdfReader, ReadStrategy};

// --- Engine-neutral model and bounded I/O ---
pub use easypdf_core::io::guards::{guard_decompression_bomb, guard_element_explosion};
pub use easypdf_core::io::repair::{RepairOptions, attempt_repair, is_likely_corrupt};
pub use easypdf_core::io::ssrf_guard::validate_url as validate_io_url;
pub use easypdf_core::{AtomicFileOutput, PdfInput, ResourceLimits};
pub use easypdf_core::{
    ImageData, ImageFormat, ListItem, PdfBlock, PdfBlockType, PdfDocumentModel, PdfPageModel,
    SourceLocation,
};

// --- Writer ---
pub use easypdf_writer::{PdfWriter, PdfWriterBuilder, WriteBackend};

// --- Manipulate ---
pub use easypdf_reader::PdfManipulator;

// --- Template ---
pub use easypdf_writer::PdfTemplateFiller;

// --- Layout crate ---
pub use easypdf_core::layout::Direction as LayoutDirection;
pub use easypdf_core::layout::FlowLayout;

// --- Markdown pipeline (optional) ---
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

// --- Table detection (optional) ---
#[cfg(feature = "markdown-table")]
pub use easypdf_markdown::table::{ColumnSeparator, TableDetectionConfig, TableDetectorProcessor};

// --- OCR pipeline (optional) ---
#[cfg(feature = "ocr")]
pub use easypdf_markdown::ocr::{
    OcrConfig, OcrEngine, OcrImage, OcrProcessor, OcrResult, OcrTrigger, WordBox,
};

// --- OCR engines (optional) ---
#[cfg(feature = "ocr")]
pub use easypdf_ocr::{
    AuthMethod as OcrAuthMethod, BackoffStrategy, HttpClientConfig as OcrHttpClientConfig,
    HttpOcrEngine, OcrHttpError, RateLimitConfig,
};

// --- Rendering (optional) ---
#[cfg(feature = "render")]
pub use easypdf_markdown::render::error::RenderError;
#[cfg(feature = "render")]
pub use easypdf_markdown::render::{
    Background, PdfRenderer, RenderBackend, RenderConfig, RenderedImage,
};

// --- Resident daemon (optional) ---
#[cfg(feature = "resident")]
pub use easypdf_runtime::resident::serve as resident_serve;
#[cfg(feature = "resident")]
pub use easypdf_runtime::resident::try_attach as resident_try_attach;
#[cfg(feature = "resident")]
pub use easypdf_runtime::resident::{AutosaveMode, ResidentClient, ResidentConfig, ResidentServer};

// --- MCP server (optional) ---
#[cfg(feature = "mcp")]
pub use easypdf_runtime::mcp::McpServer;

// ======================================================================
// Internal modules
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
#[cfg(any(test, feature = "html"))]
use html::markdown_to_html;

mod crypto_facade;
mod facade_features;

#[cfg(test)]
mod tests;

// ======================================================================
// EasyPdf facade
// ======================================================================

use std::path::{Path, PathBuf};

/// The main entry point for all `easypdf-rust` operations.
///
/// Provides static factory methods that return ergonomic builder chains
/// for creating, reading, manipulating, and filling PDFs.
pub struct EasyPdf;

impl EasyPdf {
    // --- Create ---

    /// Start building a new PDF document.
    ///
    /// Returns a [`PdfCreateBuilder`] for configuring pages, content, and metadata.
    #[must_use = "builder method"]
    pub fn create(path: impl Into<PathBuf>) -> PdfCreateBuilder {
        PdfCreateBuilder::new(path)
    }

    // --- HTML / Markdown ---

    /// Create a PDF from an HTML string (requires `html` feature and Chromium).
    ///
    /// # Errors
    ///
    /// Returns an error if Chromium is not available or the HTML cannot be rendered.
    #[cfg(feature = "html")]
    pub fn from_html(html: &str) -> crate::Result<HtmlToPdfBuilder> {
        Ok(HtmlToPdfBuilder::new(html))
    }

    /// Create a PDF from a Markdown string (requires `html` feature and Chromium).
    ///
    /// Converts Markdown -> HTML -> PDF in two stages.
    ///
    /// # Errors
    ///
    /// Returns an error if Chromium is not available or the Markdown cannot be rendered.
    #[cfg(feature = "html")]
    pub fn from_markdown(md: &str) -> crate::Result<HtmlToPdfBuilder> {
        // Simple markdown->HTML conversion for common elements
        let html = markdown_to_html(md);
        Ok(HtmlToPdfBuilder::new(&html))
    }

    // --- Read ---

    /// Start building a PDF reader for text extraction.
    ///
    /// Returns a [`PdfReadBuilder`] for configuring page ranges and extraction modes.
    #[must_use = "builder method"]
    pub fn read(path: impl Into<PathBuf>) -> PdfReadBuilder {
        PdfReadBuilder::new(path)
    }

    /// Start a PDF to Markdown export operation.
    ///
    /// The exporter parses the PDF once, applies resource limits, and atomically
    /// replaces the output only after conversion succeeds.
    #[cfg(feature = "markdown")]
    #[must_use = "builder method"]
    pub fn export_markdown(
        input: impl Into<PathBuf>,
        output: impl Into<PathBuf>,
    ) -> PdfMarkdownExportBuilder {
        PdfMarkdownExportBuilder::new(input, output)
    }

    /// Start a PDF to Markdown in-memory conversion operation.
    ///
    /// The returned builder parses the PDF once and returns both Markdown text
    /// and a structured conversion report without requiring a temporary output file.
    #[cfg(feature = "markdown")]
    #[must_use = "builder method"]
    pub fn to_markdown(input: impl Into<PathBuf>) -> PdfMarkdownBuilder {
        PdfMarkdownBuilder::new(input)
    }

    // --- Merge ---

    /// Merge multiple PDF files into a single output file.
    ///
    /// # Errors
    ///
    /// Returns an error if any input file cannot be read or the output cannot be written.
    pub fn merge(input_paths: &[impl AsRef<Path>], output: impl AsRef<Path>) -> Result<()> {
        easypdf_reader::PdfManipulator::merge_files(input_paths, output)
    }

    // --- Split ---

    /// Start building a PDF split operation.
    #[must_use = "builder method"]
    pub fn split(path: impl Into<PathBuf>) -> PdfSplitBuilder {
        PdfSplitBuilder::new(path)
    }

    // --- Manipulate ---

    /// Start building a PDF manipulation (rotate, reorder, etc.).
    #[must_use = "builder method"]
    pub fn manipulate(path: impl Into<PathBuf>) -> PdfManipulateBuilder {
        PdfManipulateBuilder::new(path)
    }

    // --- Template / Form filling ---

    /// Fill a PDF form template with data.
    ///
    /// Returns a [`PdfFillBuilder`] for configuring field values and saving.
    #[must_use = "builder method"]
    pub fn fill_form(
        template_path: impl Into<PathBuf>,
        data: &dyn easypdf_core::PdfModel,
    ) -> PdfFillBuilder {
        PdfFillBuilder::new(template_path, data)
    }
}
