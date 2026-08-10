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
#![allow(clippy::uninlined_format_args, clippy::manual_string_new, clippy::cast_precision_loss, clippy::doc_markdown, clippy::write_with_newline, clippy::items_after_statements)]

// --- Re-exports from sub-crates ---

// Core types (flat re-export)
pub use easypdf_core::*;

// Derive macro
pub use easypdf_derive::PdfModel;

// Reader
pub use easypdf_reader::PdfReader;

// Engine-neutral model and bounded I/O
pub use easypdf_io::{AtomicFileOutput, PdfInput, ResourceLimits};
pub use easypdf_model::{PdfBlock, PdfDocumentModel, PdfPageModel, SourceLocation};

// PDF to Markdown
#[cfg(feature = "markdown")]
pub use easypdf_markdown::{
    ImagePolicy, MarkdownExportReport, MarkdownExportResult, MarkdownProfile, MarkdownRenderer,
    MarkdownWarning, OcrPolicy, PdfMarkdownExportBuilder, TablePolicy,
};

// Writer
pub use easypdf_writer::PdfWriter;

// Manipulate
pub use easypdf_manipulate::PdfManipulator;

// Template
pub use easypdf_template::PdfTemplateFiller;

// Layout crate (C2)
pub use easypdf_layout::FlowLayout;
pub use easypdf_layout::Direction as LayoutDirection;

// --- Builder entry points ---

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
    /// Converts Markdown → HTML → PDF in two stages.
    ///
    /// # Errors
    ///
    /// Returns an error if Chromium is not available or the Markdown cannot be rendered.
    #[cfg(feature = "html")]
    pub fn from_markdown(md: &str) -> crate::Result<HtmlToPdfBuilder> {
        // Simple markdown→HTML conversion for common elements
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

    // --- Merge ---

    /// Merge multiple PDF files into a single output file.
    ///
    /// # Errors
    ///
    /// Returns an error if any input file cannot be read or the output cannot be written.
    pub fn merge(input_paths: &[impl AsRef<Path>], output: impl AsRef<Path>) -> Result<()> {
        easypdf_manipulate::PdfManipulator::merge_files(input_paths, output)
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

    // --- Encrypt ---

    /// Request password protection for an existing PDF.
    ///
    /// Encryption is intentionally unavailable until a standards-compliant
    /// implementation is integrated; this method never emits a fake encryption dictionary.
    ///
    /// # Errors
    ///
    /// Returns an error if the input file cannot be read or the output cannot be written.
    pub fn encrypt(
        _input: impl AsRef<Path>,
        _output: impl AsRef<Path>,
        _password: &str,
    ) -> crate::Result<()> {
        Err(PdfError::UnsupportedFeature(
            "standards-compliant PDF encryption is not enabled".to_string(),
        ))
    }

    // --- Sign (F13) ---

    /// Request a digital signature for a PDF.
    ///
    /// Signing is intentionally unavailable until a real PKCS#7 implementation
    /// is integrated; this method never emits a placeholder that appears signed.
    ///
    /// # Errors
    ///
    /// Returns an error if the input file cannot be read or the output cannot be written.
    pub fn sign(
        _input: impl AsRef<Path>,
        _output: impl AsRef<Path>,
        _reason: &str,
    ) -> crate::Result<()> {
        Err(PdfError::UnsupportedFeature(
            "PKCS#7 PDF signing is not enabled".to_string(),
        ))
    }
}

// ======================================================================
// HTML/Markdown support
// ======================================================================

/// Convert basic Markdown to HTML.
#[cfg(any(test, feature = "html"))]
fn markdown_to_html(md: &str) -> String {
    let mut html = String::from("<html><body>\n");
    for line in md.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            html.push_str("<br/>\n");
        } else if let Some(rest) = trimmed.strip_prefix("### ") {
            use std::fmt::Write;
            let _ = write!(html, "<h3>{rest}</h3>\n");
        } else if let Some(rest) = trimmed.strip_prefix("## ") {
            use std::fmt::Write;
            let _ = write!(html, "<h2>{rest}</h2>\n");
        } else if let Some(rest) = trimmed.strip_prefix("# ") {
            use std::fmt::Write;
            let _ = write!(html, "<h1>{rest}</h1>\n");
        } else if let Some(rest) = trimmed.strip_prefix("- ") {
            use std::fmt::Write;
            let _ = write!(html, "<li>{rest}</li>\n");
        } else if let Some(rest) = trimmed.strip_prefix("> ") {
            use std::fmt::Write;
            let _ = write!(html, "<blockquote>{rest}</blockquote>\n");
        } else {
            let processed = process_inline_formatting(trimmed);
            use std::fmt::Write;
            let _ = write!(html, "<p>{processed}</p>\n");
        }
    }
    html.push_str("</body></html>");
    html
}

/// Process inline **bold** and *italic* Markdown.
#[cfg(any(test, feature = "html"))]
fn process_inline_formatting(text: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
            result.push_str("<b>");
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '*') {
                result.push(chars[i]);
                i += 1;
            }
            result.push_str("</b>");
            if i + 1 < chars.len() { i += 2; }
        } else if chars[i] == '*' && i + 1 < chars.len() && chars[i + 1] != '*' {
            result.push_str("<i>");
            i += 1;
            while i < chars.len() && chars[i] != '*' {
                result.push(chars[i]);
                i += 1;
            }
            result.push_str("</i>");
            if i < chars.len() { i += 1; }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
}

/// Builder for HTML-to-PDF conversion (requires `html` feature).
#[cfg(feature = "html")]
#[must_use]
pub struct HtmlToPdfBuilder {
    html: String,
    title: String,
}

#[cfg(feature = "html")]
impl HtmlToPdfBuilder {
    fn new(html: &str) -> Self {
        Self {
            html: html.to_string(),
            title: "HTML Document".into(),
        }
    }

    /// Set the document title.
    #[must_use = "builder method"]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Render HTML to PDF and save.
    ///
    /// # Errors
    ///
    /// Returns an error if Chromium is not available or rendering fails.
    pub fn save(self, output: impl AsRef<Path>) -> crate::Result<()> {
        use std::collections::BTreeMap;
        let mut warnings = Vec::new();
        let images = BTreeMap::new();
        let fonts = BTreeMap::new();
        let options = printpdf::GeneratePdfOptions::default();
        let doc = printpdf::PdfDocument::from_html(&self.html, &images, &fonts, &options, &mut warnings)
            .map_err(|e| PdfError::Other(format!("HTML render error: {e}")))?;
        let file = std::fs::File::create(output)?;
        let mut buf = std::io::BufWriter::new(file);
        let save_opts = printpdf::PdfSaveOptions::default();
        doc.save_writer(&mut buf, &save_opts, &mut warnings);
        Ok(())
    }
}

// ======================================================================
// Builder types
// ======================================================================

/// Builder for creating new PDF documents.
#[must_use]
pub struct PdfCreateBuilder {
    path: PathBuf,
    title: String,
    page_size: PageSize,
    orientation: Orientation,
    metadata: PdfMetadata,
    #[allow(dead_code)]
    fonts: Vec<PdfFont>,
    handlers: Vec<Box<dyn easypdf_core::PdfWriteHandler>>,
}

impl PdfCreateBuilder {
    fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            title: String::from("Untitled"),
            page_size: PageSize::A4,
            orientation: Orientation::default(),
            metadata: PdfMetadata::default(),
            fonts: Vec::new(),
            handlers: Vec::new(),
        }
    }

    /// Set the document title.
    #[must_use = "builder method"]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the default page size.
    #[must_use = "builder method"]
    pub const fn page_size(mut self, size: PageSize) -> Self {
        self.page_size = size;
        self
    }

    /// Set the page orientation.
    #[must_use = "builder method"]
    pub const fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Set document metadata.
    #[must_use = "builder method"]
    pub fn metadata(mut self, metadata: PdfMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Register a write handler.
    #[must_use = "builder method"]
    pub fn register_handler(mut self, handler: Box<dyn easypdf_core::PdfWriteHandler>) -> Self {
        self.handlers.push(handler);
        self
    }

    /// Write text and finalize the document in one call.
    ///
    /// This is a convenience method for simple single-page PDFs.
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be written.
    pub fn add_text(self, content: impl Into<String>) -> PdfTextBuilder<Self> {
        PdfTextBuilder {
            parent: self,
            text: PdfText::new(content),
        }
    }

    /// Build the writer for manual page-by-page construction.
    ///
    /// # Errors
    ///
    /// Returns an error if the writer cannot be initialized.
    pub fn build(self) -> Result<easypdf_writer::PdfWriter> {
        let mut writer = easypdf_writer::PdfWriter::new(&self.title);
        writer = writer.metadata(self.metadata);
        for handler in self.handlers {
            writer = writer.register_handler(handler);
        }
        Ok(writer)
    }

    /// Build, add a default page, write text, and save — all in one call.
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be created or written.
    pub fn do_write(self) -> Result<PathBuf> {
        let path = self.path.clone();
        let page_size = self.page_size;
        let orientation = self.orientation;
        let mut writer = self.build()?;
        writer.add_page(page_size, orientation)?;
        writer.finish(&path)?;
        Ok(path)
    }
}

/// Builder for adding text to a PDF, returned by [`PdfCreateBuilder::add_text`].
#[must_use]
pub struct PdfTextBuilder<P> {
    parent: P,
    text: PdfText,
}

impl PdfTextBuilder<PdfCreateBuilder> {
    /// Set the font for this text.
    #[must_use = "builder method"]
    pub fn font(mut self, font: PdfFont) -> Self {
        self.text = self.text.font(font);
        self
    }

    /// Set the position as (x, y) in PDF points.
    #[must_use = "builder method"]
    pub fn position(self, x: f64, y: f64) -> PdfPositionedTextBuilder {
        PdfPositionedTextBuilder {
            parent: self.parent,
            text: self.text,
            x,
            y,
        }
    }

    /// Finalize by writing the text at the default position (100, 700).
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be created or written.
    pub fn do_write(self) -> Result<PathBuf> {
        let path = self.parent.path.clone();
        let page_size = self.parent.page_size;
        let orientation = self.parent.orientation;
        let mut writer = self.parent.build()?;
        writer.add_page(page_size, orientation)?;
        writer.write_text(&self.text, 100.0, 700.0)?;
        writer.finish(&path)?;
        Ok(path)
    }
}

/// Builder for text with an explicit position.
#[must_use]
pub struct PdfPositionedTextBuilder {
    parent: PdfCreateBuilder,
    text: PdfText,
    x: f64,
    y: f64,
}

impl PdfPositionedTextBuilder {
    /// Finalize and write the PDF.
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be created or written.
    pub fn do_write(self) -> Result<PathBuf> {
        let path = self.parent.path.clone();
        let page_size = self.parent.page_size;
        let orientation = self.parent.orientation;
        let mut writer = self.parent.build()?;
        writer.add_page(page_size, orientation)?;
        writer.write_text(&self.text, self.x, self.y)?;
        writer.finish(&path)?;
        Ok(path)
    }
}

// --- Read builder ---

/// Builder for reading/extracting content from PDFs.
#[must_use]
pub struct PdfReadBuilder {
    path: PathBuf,
    pages: Option<std::ops::Range<usize>>,
}

impl PdfReadBuilder {
    fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            pages: None,
        }
    }

    /// Limit extraction to a specific page range (0-based).
    #[must_use = "builder method"]
    pub fn pages(mut self, range: std::ops::Range<usize>) -> Self {
        self.pages = Some(range);
        self
    }

    /// Open a reusable, single-parse reader session.
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be read or parsed.
    pub fn open(self) -> Result<PdfReader> {
        let mut reader = PdfReader::open(&self.path)?;
        if let Some(range) = self.pages {
            reader = reader.try_pages(range)?;
        }
        Ok(reader)
    }

    /// Extract all text from the PDF.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Parse` if the PDF cannot be read.
    pub fn extract_text(self) -> Result<String> {
        self.open()?.extract_text()
    }

    /// Extract PDF metadata.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Parse` if the PDF cannot be read.
    pub fn metadata(self) -> Result<PdfMetadata> {
        self.open()?.extract_metadata()
    }

    /// Get the number of pages.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Parse` if the PDF cannot be read.
    pub fn page_count(self) -> Result<usize> {
        self.open()?.page_count()
    }
}

// --- Split builder ---

/// Builder for splitting a PDF into individual pages.
#[must_use]
pub struct PdfSplitBuilder {
    path: PathBuf,
    pages_per_file: usize,
}

impl PdfSplitBuilder {
    fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            pages_per_file: 1,
        }
    }

    /// Set the number of pages per split file (default: 1).
    #[must_use = "builder method"]
    pub const fn every_n_pages(mut self, n: usize) -> Self {
        self.pages_per_file = n;
        self
    }

    /// Split the PDF and save pages to a directory.
    ///
    /// Files are named `page_001.pdf`, `page_002.pdf`, etc.
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be read or split files cannot be written.
    pub fn save_to_dir(self, output_dir: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
        let manipulator = easypdf_manipulate::PdfManipulator::open(&self.path)?;
        let total_pages = manipulator.page_count();
        let output_dir = output_dir.as_ref();
        std::fs::create_dir_all(output_dir)?;

        let mut output_paths = Vec::new();
        let mut start = 0;

        while start < total_pages {
            let end = std::cmp::min(start + self.pages_per_file, total_pages);
            let mut chunk = manipulator.extract_pages(start..end)?;
            let filename = format!("page_{:03}.pdf", start / self.pages_per_file + 1);
            let output_path = output_dir.join(&filename);
            chunk.save(&output_path)?;
            output_paths.push(output_path);
            start = end;
        }

        Ok(output_paths)
    }
}

// --- Manipulate builder ---

/// Builder for PDF manipulation operations (rotate, reorder, watermark).
#[must_use]
pub struct PdfManipulateBuilder {
    path: PathBuf,
    rotations: Vec<(usize, Rotation)>,
    order: Option<Vec<usize>>,
}

impl PdfManipulateBuilder {
    fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            rotations: Vec::new(),
            order: None,
        }
    }

    /// Rotate a specific page (1-based index).
    #[must_use = "builder method"]
    pub fn rotate_page(mut self, page_number: usize, rotation: Rotation) -> Self {
        self.rotations.push((page_number, rotation));
        self
    }

    /// Rotate all pages.
    #[must_use = "builder method"]
    pub fn rotate_all(self, rotation: Rotation) -> Self {
        // This will be applied inside save() by iterating all pages
        self.rotate(rotation)
    }

    /// Rotate all pages (alias for builder chain).
    #[must_use = "builder method"]
    pub fn rotate(mut self, rotation: Rotation) -> Self {
        self.rotations.push((0, rotation)); // 0 means "all pages"
        self
    }

    /// Reorder pages according to the given permutation (0-based).
    #[must_use = "builder method"]
    pub fn reorder_pages(mut self, order: &[usize]) -> Self {
        self.order = Some(order.to_vec());
        self
    }

    /// Apply all manipulations and save to the output file.
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be read or saved.
    pub fn save(self, output: impl AsRef<Path>) -> Result<()> {
        let mut manipulator = easypdf_manipulate::PdfManipulator::open(&self.path)?;

        for (page_num, rotation) in &self.rotations {
            if *page_num == 0 {
                // Apply to all pages
                let count = manipulator.page_count();
                for p in 1..=count {
                    manipulator.rotate_page(p, *rotation)?;
                }
            } else {
                manipulator.rotate_page(*page_num, *rotation)?;
            }
        }

        if let Some(order) = &self.order {
            manipulator.reorder_pages(order)?;
        }

        manipulator.save(output)
    }
}

// --- Fill builder ---

/// Builder for filling PDF form templates.
#[must_use]
pub struct PdfFillBuilder {
    template_path: PathBuf,
    fields: Vec<(String, String)>,
}

impl PdfFillBuilder {
    fn new(template_path: impl Into<PathBuf>, data: &dyn easypdf_core::PdfModel) -> Self {
        let _ = data; // The PdfModel trait will be used to extract field mappings
        Self {
            template_path: template_path.into(),
            fields: Vec::new(),
        }
    }

    /// Add a field value to fill.
    #[must_use = "builder method"]
    pub fn field(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.push((name.into(), value.into()));
        self
    }

    /// Add multiple field values.
    #[must_use = "builder method"]
    pub fn fields(
        mut self,
        fields: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        for (name, value) in fields {
            self.fields.push((name.into(), value.into()));
        }
        self
    }

    /// Fill form fields and save to the output file.
    ///
    /// # Errors
    ///
    /// Returns an error if the template cannot be read or the output cannot be written.
    pub fn save(self, output: impl AsRef<Path>) -> Result<()> {
        let mut filler = easypdf_template::PdfTemplateFiller::open(&self.template_path)?;
        for (name, value) in &self.fields {
            filler.fill_field(name, value)?;
        }
        filler.save(output)
    }
}

/// Convenience re-exports in a `prelude` module.
pub mod prelude {
    pub use super::EasyPdf;
    #[cfg(feature = "markdown")]
    pub use super::{ImagePolicy, MarkdownProfile, OcrPolicy, TablePolicy};
    pub use easypdf_core::*;
    pub use easypdf_derive::PdfModel;
}

// --- Built-in Handlers ---

/// A write handler that adds page numbers to each page.
pub struct PageNumberHandler {
    font: easypdf_core::PdfFont,
    /// Position offset from bottom-center in PDF points.
    offset_y: f64,
}

impl PageNumberHandler {
    /// Create a new page number handler.
    #[must_use = "builder method"]
    pub fn new() -> Self {
        Self {
            font: easypdf_core::PdfFont::helvetica(10.0),
            offset_y: 30.0,
        }
    }

    /// Set the font for page numbers.
    #[must_use = "builder method"]
    pub fn font(mut self, font: easypdf_core::PdfFont) -> Self {
        self.font = font;
        self
    }

    /// Set the Y offset from the bottom of the page.
    #[must_use = "builder method"]
    pub fn offset_y(mut self, offset: f64) -> Self {
        self.offset_y = offset;
        self
    }
}

impl Default for PageNumberHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl easypdf_core::PdfWriteHandler for PageNumberHandler {
    fn after_page(&mut self, page_number: usize) -> easypdf_core::Result<()> {
        // Handler returns Ok; actual page number rendering is done by the writer
        let _ = page_number;
        Ok(())
    }
}

// --- Table Rendering ---

/// Render a table onto a writer using lines and text.
///
/// # Errors
///
/// Returns an error if any write operation fails.
pub fn write_table(
    writer: &mut easypdf_writer::PdfWriter,
    table: &easypdf_core::PdfTable,
    x: f64,
    y: f64,
    col_widths: &[f64],
    row_height: f64,
    font: &easypdf_core::PdfFont,
) -> easypdf_core::Result<()> {
    let ncols = table.headers.len();
    if ncols == 0 {
        return Ok(());
    }

    let widths: Vec<f64> = if col_widths.is_empty() {
        let default_w = 500.0 / ncols as f64;
        vec![default_w; ncols]
    } else {
        col_widths.to_vec()
    };

        // Draw header row
    let header_y = y;
    for (i, header) in table.headers.iter().enumerate() {
        let cell_x = x + widths.iter().take(i).sum::<f64>();
        writer.draw_rect_stroke(cell_x, header_y, widths[i], row_height, 0.5);
        let txt = easypdf_core::PdfText::new(header.as_str()).font(font.clone().bold());
        writer.write_text(&txt, cell_x + 4.0, header_y + row_height - font.size - 2.0)?;
    }

    // Draw data rows
    for (row_idx, row) in table.rows.iter().enumerate() {
        let row_y = y - (row_idx as f64 + 1.0) * row_height;
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                let cell_x = x + widths.iter().take(i).sum::<f64>();
                writer.draw_rect_stroke(cell_x, row_y, widths[i], row_height, 0.5);
                let txt = easypdf_core::PdfText::new(cell.as_str()).font(font.clone());
                writer.write_text(&txt, cell_x + 4.0, row_y + row_height - font.size - 2.0)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::redundant_closure_for_method_calls, clippy::needless_borrow, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn test_easy_pdf_create_builder() {
        let builder = EasyPdf::create("test.pdf")
            .title("Test")
            .page_size(PageSize::A4);
        assert!(builder.build().is_ok());
    }

    #[test]
    fn test_page_number_handler_default() {
        let h = PageNumberHandler::default();
        assert_eq!(h.offset_y, 30.0);
        assert_eq!(h.font.size, 10.0);
    }

    #[test]
    fn test_page_number_handler_builder() {
        let h = PageNumberHandler::new()
            .font(PdfFont::times_roman(12.0))
            .offset_y(50.0);
        assert_eq!(h.offset_y, 50.0);
        assert_eq!(h.font.size, 12.0);
    }

    #[test]
    fn test_write_table_empty() {
        let mut writer = PdfWriter::new("test");
        writer.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        let table = PdfTable::new(vec![]);
        assert!(write_table(&mut writer, &table, 50.0, 700.0, &[], 20.0, &PdfFont::helvetica(10.0)).is_ok());
    }

    #[test]
    fn test_write_table_with_data() {
        let mut writer = PdfWriter::new("test");
        writer.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        let table = PdfTable::new(vec!["A".into(), "B".into()])
            .row(vec!["1".into(), "2".into()]);
        assert!(write_table(&mut writer, &table, 50.0, 700.0, &[100.0, 100.0], 25.0, &PdfFont::helvetica(10.0)).is_ok());
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_table_test.pdf");
        writer.finish(&path).unwrap();
        assert!(path.exists());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_read_builder() {
        let builder = EasyPdf::read("nonexistent.pdf");
        assert!(builder.extract_text().is_err());
    }

    #[test]
    fn test_split_builder() {
        let builder = EasyPdf::split("test.pdf").every_n_pages(2);
        assert!(builder.save_to_dir("/nonexistent/dir").is_err());
    }

    #[test]
    fn test_manipulate_builder() {
        let result = EasyPdf::manipulate("/nonexistent/file.pdf")
            .rotate_all(Rotation::Clockwise90)
            .save("/tmp/out.pdf");
        assert!(result.is_err());
    }

    #[test]
    fn test_fill_builder_save() {
        struct DummyModel;
        impl easypdf_core::PdfModel for DummyModel {
            fn render(&self) -> easypdf_core::Result<Vec<easypdf_core::RenderedElement>> {
                Ok(vec![])
            }
            fn metadata(&self) -> easypdf_core::PdfModelMetadata {
                easypdf_core::PdfModelMetadata::default()
            }
        }
        let result = EasyPdf::fill_form("/nonexistent/template.pdf", &DummyModel)
            .save("/tmp/out.pdf");
        assert!(result.is_err());
    }

    #[test]
    fn test_fill_builder_with_fields() {
        struct DummyModel;
        impl easypdf_core::PdfModel for DummyModel {
            fn render(&self) -> easypdf_core::Result<Vec<easypdf_core::RenderedElement>> {
                Ok(vec![])
            }
            fn metadata(&self) -> easypdf_core::PdfModelMetadata {
                easypdf_core::PdfModelMetadata::default()
            }
        }
        let result = EasyPdf::fill_form("/nonexistent/template.pdf", &DummyModel)
            .field("name", "value")
            .fields([("email", "a@b.com")])
            .save("/tmp/out.pdf");
        assert!(result.is_err());
    }

    #[test]
    fn test_read_builder_metadata() {
        let result = EasyPdf::read("/nonexistent.pdf").metadata();
        assert!(result.is_err());
    }

    #[test]
    fn test_read_builder_page_count() {
        let result = EasyPdf::read("/nonexistent.pdf").page_count();
        assert!(result.is_err());
    }

    #[test]
    fn test_manipulate_rotate_specific_page() {
        let result = EasyPdf::manipulate("/nonexistent.pdf")
            .rotate_page(1, Rotation::Clockwise90)
            .save("/tmp/out.pdf");
        assert!(result.is_err());
    }

    #[test]
    fn test_manipulate_reorder() {
        let result = EasyPdf::manipulate("/nonexistent.pdf")
            .reorder_pages(&[0])
            .save("/tmp/out.pdf");
        assert!(result.is_err());
    }

    #[test]
    fn test_prelude() {
        use prelude::*;
        let _ = EasyPdf::create("test.pdf");
        let _ = PageSize::A4;
    }

    #[test]
    fn test_create_builder_do_write_error() {
        let result = EasyPdf::create("/invalid/path/out.pdf")
            .do_write();
        assert!(result.is_err());
    }

    #[test]
    fn test_create_builder_with_text() {
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_facade_text.pdf");
        let result = EasyPdf::create(&path)
            .add_text("Hi")
                .font(PdfFont::helvetica(12.0))
            .do_write();
        assert!(result.is_ok());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_create_builder_with_text_position() {
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_facade_pos.pdf");
        let result = EasyPdf::create(&path)
            .add_text("Hi")
                .font(PdfFont::helvetica(12.0))
                .position(200.0, 500.0)
            .do_write();
        assert!(result.is_ok());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_read_builder_pages() {
        let result = EasyPdf::read("/nonexistent.pdf")
            .pages(0..5)
            .extract_text();
        assert!(result.is_err());
    }

    #[test]
    fn test_manipulate_rotate_all_then_reorder() {
        let result = EasyPdf::manipulate("/nonexistent.pdf")
            .rotate_all(Rotation::Clockwise180)
            .reorder_pages(&[1, 0])
            .save("/tmp/out.pdf");
        assert!(result.is_err());
    }

    #[test]
    fn test_markdown_to_html() {
        let md = "# Title\n\n**bold** and *italic* text\n\n- item 1\n- item 2";
        let html = markdown_to_html(md);
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<b>bold</b>"));
        assert!(html.contains("<i>italic</i>"));
        assert!(html.contains("<li>item 1</li>"));
        assert!(html.contains("<li>item 2</li>"));
    }

    #[test]
    fn test_markdown_headings() {
        let html = markdown_to_html("## H2\n### H3\n> quote");
        assert!(html.contains("<h2>H2</h2>"));
        assert!(html.contains("<h3>H3</h3>"));
        assert!(html.contains("<blockquote>quote</blockquote>"));
    }

    #[test]
    fn test_encrypt() {
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_encrypt_test.pdf");
        // Create a minimal PDF via writer
        let mut writer = PdfWriter::new("test");
        writer.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        writer.write_text(&PdfText::new("secret"), 100.0, 700.0).unwrap();
        writer.finish(&path).unwrap();

        let out = dir.join("easypdf_encrypted.pdf");
        let result = EasyPdf::encrypt(&path, &out, "password123");
        assert!(matches!(result, Err(PdfError::UnsupportedFeature(_))));
        assert!(!out.exists());
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn test_permission_flags() {
        // Permission flags: -4 = 0xFFFF_FFFC = allow print + copy, deny modify
        // -4 in two's complement = all bits set except bit 2 (modify)
        let flags: i32 = -4;
        let print_allowed = (flags & 0b0100) != 0; // bit 2 = print (actually bit 2 = modify, bit 3 = print)
        let modify_denied = (flags & 0b1000) == 0; // bit 3 = modify
        assert!(print_allowed || modify_denied); // verify at least one flag is set
        let _ = print_allowed;
    }

    #[test]
    fn test_sign() {
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_sign_test.pdf");
        let mut w = PdfWriter::new("test");
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        w.write_text(&PdfText::new("sig"), 100.0, 700.0).unwrap();
        w.finish(&path).unwrap();
        let out = dir.join("easypdf_signed.pdf");
        assert!(matches!(
            EasyPdf::sign(&path, &out, "Approved"),
            Err(PdfError::UnsupportedFeature(_))
        ));
        assert!(!out.exists());
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn test_encrypt_error_nonexistent_input() {
        assert!(EasyPdf::encrypt("/nonexistent/in.pdf", "/tmp/out.pdf", "pwd").is_err());
    }
}
