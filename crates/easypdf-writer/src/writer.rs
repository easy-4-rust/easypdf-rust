//! Main `PdfWriter` struct and core PDF writing methods.
//!
//! Backed by `printpdf` for PDF construction. Supports two write backends:
//! - **In-memory** (default): the entire document is built in memory.
//! - **Spill**: finalized pages are serialized to temp files, bounding peak memory.

use easypdf_core::AtomicFileOutput;
use easypdf_core::error::{PdfError, Result};
use easypdf_core::handler_chain::{PRIORITY_NORMAL, WriteHandlerChain};
use easypdf_core::layout::LayoutSink;
use easypdf_core::{
    FontFamily, Orientation, PageSize, PdfColor, PdfFont, PdfImage, PdfMetadata, PdfText,
    PdfWriteHandler,
};
use printpdf::{Mm, Op, PdfDocument, PdfFontHandle, PdfPage, PdfSaveOptions, Point, Pt, TextItem};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use crate::backend::{PageSpillWriter, SpilledPageData, WriteBackend};
use crate::font::map_builtin_font;

/// PDF measurement units.
const PT_TO_MM: f64 = 25.4 / 72.0;
/// Default margin in points for auto-positioned text.
const DEFAULT_MARGIN: f64 = 72.0;

/// A writer for creating new PDF documents.
///
/// Builds pages from operations, then serializes the document to bytes.
/// Supports multiple pages, images, custom fonts, and shapes.
///
/// # Write backends
///
/// Use [`WriteBackend`] to choose between in-memory and page-level spill modes.
/// For large documents, the spill backend bounds peak memory by serializing
/// finalized pages to temporary files.
///
/// # Handler chain
///
/// Handlers are managed by a priority-sorted [`WriteHandlerChain`]. The
/// [`register_handler`](Self::register_handler) method uses
/// [`PRIORITY_NORMAL`](easypdf_core::handler_chain::PRIORITY_NORMAL); use
/// [`register_handler_with_priority`](Self::register_handler_with_priority)
/// for custom priorities.
///
/// # Examples
///
/// ```
/// use easypdf_writer::{PdfWriter, PdfWriterBuilder, WriteBackend};
/// use easypdf_core::*;
///
/// // Simple construction (backward-compatible).
/// let w = PdfWriter::new("title");
///
/// // Builder with spill backend.
/// let w = PdfWriterBuilder::new("Big Report")
///     .backend(WriteBackend::auto(500))
///     .build()
///     .unwrap();
/// ```
pub struct PdfWriter {
    pub(crate) doc: PdfDocument,
    /// Accumulated completed pages (in-memory mode only).
    pages: Vec<PdfPage>,
    /// Operations being built for the current page.
    pub(crate) current_page_ops: Vec<Op>,
    /// Current page size for the page being built.
    current_page_size: (f64, f64),
    /// Current page number (1-based).
    current_page_number: usize,
    /// Whether the current page still accepts content and awaits finalization.
    current_page_open: bool,
    /// Whether the document lifecycle has started.
    document_started: bool,
    /// Registered custom font IDs keyed by path.
    custom_fonts: HashMap<String, printpdf::FontId>,
    /// Document metadata.
    pub(crate) metadata: PdfMetadata,
    /// Priority-sorted handler chain.
    chain: WriteHandlerChain,
    /// Auto-cursor for add_text convenience.
    text_cursor: (f64, f64),
    /// Output stream for flush-based writing.
    output: Option<Box<dyn Write>>,
    /// Write backend configuration.
    backend: WriteBackend,
    /// Page-level spill writer (active when backend is `Spill`).
    spill_writer: Option<PageSpillWriter>,
}

impl PdfWriter {
    /// Create a new PDF document (writes to file via `finish`).
    ///
    /// Uses the default in-memory backend. For advanced configuration,
    /// use [`PdfWriterBuilder`](crate::PdfWriterBuilder).
    #[must_use]
    pub fn new(title: &str) -> Self {
        Self {
            doc: PdfDocument::new(title),
            pages: Vec::new(),
            current_page_ops: Vec::new(),
            current_page_size: PageSize::A4.dimensions(),
            current_page_number: 0,
            current_page_open: false,
            document_started: false,
            custom_fonts: HashMap::new(),
            metadata: PdfMetadata::default(),
            chain: WriteHandlerChain::new(),
            text_cursor: (DEFAULT_MARGIN, 0.0),
            output: None,
            backend: WriteBackend::default(),
            spill_writer: None,
        }
    }

    /// Create a new PDF document that writes to a generic writer (hutool pattern).
    #[must_use]
    pub fn new_from_writer(writer: impl Write + 'static) -> Self {
        let mut s = Self::new("untitled");
        s.output = Some(Box::new(writer));
        s
    }

    /// Internal constructor used by [`PdfWriterBuilder`].
    ///
    /// # Errors
    ///
    /// Returns an error if the spill backend cannot be initialized.
    pub(crate) fn with_config(
        title: &str,
        metadata: PdfMetadata,
        backend: WriteBackend,
        chain: WriteHandlerChain,
    ) -> Result<Self> {
        let spill_writer = match &backend {
            WriteBackend::Spill {
                spill_dir,
                compress,
                threshold_pages,
            } => Some(PageSpillWriter::new(
                spill_dir.clone(),
                *compress,
                *threshold_pages,
            )?),
            WriteBackend::InMemory => None,
        };

        Ok(Self {
            doc: PdfDocument::new(title),
            pages: Vec::new(),
            current_page_ops: Vec::new(),
            current_page_size: PageSize::A4.dimensions(),
            current_page_number: 0,
            current_page_open: false,
            document_started: false,
            custom_fonts: HashMap::new(),
            metadata,
            chain,
            text_cursor: (DEFAULT_MARGIN, 0.0),
            output: None,
            backend,
            spill_writer,
        })
    }

    /// Set document metadata.
    #[must_use]
    pub fn metadata(mut self, metadata: PdfMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Register a write handler with default priority
    /// ([`PRIORITY_NORMAL`](easypdf_core::handler_chain::PRIORITY_NORMAL)).
    #[must_use]
    pub fn register_handler(mut self, handler: Box<dyn PdfWriteHandler>) -> Self {
        self.chain.register(handler, PRIORITY_NORMAL);
        self
    }

    /// Register a write handler with a specific execution priority.
    ///
    /// Lower priority values execute first.
    #[must_use]
    pub fn register_handler_with_priority(
        mut self,
        handler: Box<dyn PdfWriteHandler>,
        priority: f64,
    ) -> Self {
        self.chain.register(handler, priority);
        self
    }

    /// Register a custom TTF/OTF font from a file path.
    pub fn register_font_from_path(&mut self, path: &str) -> Result<String> {
        let font_data = std::fs::read(path)?;
        self.register_font_from_bytes(path, &font_data)
    }

    /// Register a custom TTF/OTF font from bytes.
    pub fn register_font_from_bytes(&mut self, key: &str, font_data: &[u8]) -> Result<String> {
        let mut warnings = Vec::new();
        let parsed = printpdf::ParsedFont::from_bytes(font_data, 0, &mut warnings)
            .ok_or_else(|| PdfError::Parse(format!("Failed to parse font: {key}")))?;
        let font_id = self.doc.add_font(&parsed);
        self.custom_fonts.insert(key.to_string(), font_id);
        Ok(key.to_string())
    }

    /// Write text using a custom (non-builtin) font.
    pub fn write_text_with_custom_font(
        &mut self,
        text: &str,
        font_key: &str,
        font_size: f64,
        x_pt: f64,
        y_pt: f64,
    ) -> Result<()> {
        let font_id = self.custom_fonts.get(font_key).cloned().ok_or_else(|| {
            PdfError::UnsupportedFeature(format!("Custom font '{font_key}' not registered."))
        })?;
        let pos = Point {
            x: Pt(x_pt as f32),
            y: Pt(y_pt as f32),
        };
        let ops = vec![
            Op::StartTextSection,
            Op::SetTextCursor { pos },
            Op::SetFont {
                font: PdfFontHandle::External(font_id),
                size: Pt(font_size as f32),
            },
            Op::ShowText {
                items: vec![TextItem::Text(text.to_string())],
            },
            Op::EndTextSection,
        ];
        self.current_page_ops.extend(ops);
        Ok(())
    }

    /// Add a new page.
    pub fn add_page(&mut self, size: PageSize, orientation: Orientation) -> Result<usize> {
        self.finalize_current_page()?;
        self.ensure_document_started()?;
        self.current_page_number += 1;
        let (width, height) = size.dimensions();
        self.current_page_size = match orientation {
            Orientation::Portrait => (width, height),
            Orientation::Landscape => (height, width),
        };
        self.text_cursor = (DEFAULT_MARGIN, self.current_page_size.1 - DEFAULT_MARGIN);
        self.chain.before_page(self.current_page_number)?;
        self.current_page_open = true;
        Ok(self.current_page_number)
    }

    fn finalize_current_page(&mut self) -> Result<()> {
        if !self.current_page_open {
            return Ok(());
        }
        self.chain.after_page(self.current_page_number)?;
        let ops = std::mem::take(&mut self.current_page_ops);
        let (w, h) = self.current_page_size;

        // If spill writer is active, attempt to spill this page.
        if let Some(ref mut spill) = self.spill_writer {
            let page_data = SpilledPageData {
                page_number: self.current_page_number,
                width_pt: w,
                height_pt: h,
                ops: ops.clone(),
            };
            if spill.maybe_spill(&page_data)?.is_some() {
                // Page was spilled -- do not keep in memory.
                self.current_page_open = false;
                return Ok(());
            }
        }

        // Keep page in memory (in-memory mode, or below spill threshold).
        self.pages.push(PdfPage::new(
            Mm(w as f32 * PT_TO_MM as f32),
            Mm(h as f32 * PT_TO_MM as f32),
            ops,
        ));
        self.current_page_open = false;
        Ok(())
    }

    fn ensure_document_started(&mut self) -> Result<()> {
        if self.document_started {
            return Ok(());
        }
        self.chain.before_document()?;
        self.document_started = true;
        Ok(())
    }

    /// Get current page number (1-based).
    #[must_use]
    pub const fn current_page_number(&self) -> usize {
        self.current_page_number
    }

    /// Get total finalized pages.
    #[must_use]
    pub fn page_count(&self) -> usize {
        // Include both in-memory pages and spilled pages.
        let spilled = self.spill_writer.as_ref().map_or(0, |s| s.spilled_count());
        self.pages.len() + spilled
    }

    /// Return whether this writer is in constant-memory (spill) mode.
    #[must_use]
    pub fn is_constant_memory(&self) -> bool {
        self.backend.is_constant_memory()
    }

    /// Switch to or from constant-memory mode.
    ///
    /// When enabled, the backend is set to [`WriteBackend::constant_memory()`]
    /// which spills every page immediately after finalization. When disabled,
    /// the backend is set to [`WriteBackend::InMemory`].
    ///
    /// Note: switching mode mid-document has no effect on already-finalized pages.
    pub fn set_constant_memory(&mut self, enabled: bool) {
        if enabled {
            if !self.backend.is_constant_memory() {
                self.backend = WriteBackend::constant_memory();
                // Initialize spill writer if not present.
                if self.spill_writer.is_none() {
                    self.spill_writer = PageSpillWriter::new(None, true, 1).ok();
                }
            }
        } else {
            self.backend = WriteBackend::InMemory;
            // We do not drop the spill writer -- already-spilled pages need
            // to be collected at finish time.
        }
    }

    /// Return the number of registered handlers.
    #[must_use]
    pub fn handler_count(&self) -> usize {
        self.chain.len()
    }

    /// Return the document title from metadata, if set.
    #[must_use]
    pub fn metadata_title(&self) -> Option<&str> {
        self.metadata.title.as_deref()
    }

    /// Write text at (x, y) in PDF points.
    pub fn write_text(&mut self, text: &PdfText, x_pt: f64, y_pt: f64) -> Result<()> {
        if let FontFamily::Custom(ref path) = text.font.family
            && let Some(font_id) = self.custom_fonts.get(path.as_ref())
        {
            let pos = Point {
                x: Pt(x_pt as f32),
                y: Pt(y_pt as f32),
            };
            let ops = vec![
                Op::StartTextSection,
                Op::SetTextCursor { pos },
                Op::SetFont {
                    font: PdfFontHandle::External(font_id.clone()),
                    size: Pt(text.font.size as f32),
                },
                Op::ShowText {
                    items: vec![TextItem::Text(text.content.clone())],
                },
                Op::EndTextSection,
            ];
            self.current_page_ops.extend(ops);
            return Ok(());
        }
        let bf = map_builtin_font(&text.font);
        let pos = Point {
            x: Pt(x_pt as f32),
            y: Pt(y_pt as f32),
        };
        let ops = vec![
            Op::StartTextSection,
            Op::SetTextCursor { pos },
            Op::SetFont {
                font: PdfFontHandle::Builtin(bf),
                size: Pt(text.font.size as f32),
            },
            Op::ShowText {
                items: vec![TextItem::Text(text.content.clone())],
            },
            Op::EndTextSection,
        ];
        self.current_page_ops.extend(ops);
        Ok(())
    }

    /// Add auto-positioned text (hutool addText pattern).
    pub fn add_text(&mut self, font: &PdfFont, text: &str) -> Result<&mut Self> {
        let (x, y) = self.text_cursor;
        self.write_text(&PdfText::new(text).font(font.clone()), x, y)?;
        self.text_cursor.1 -= font.size + 4.0;
        Ok(self)
    }

    /// Add auto-positioned text with explicit color.
    pub fn add_text_colored(
        &mut self,
        font: &PdfFont,
        color: &PdfColor,
        text: &str,
    ) -> Result<&mut Self> {
        let (x, y) = self.text_cursor;
        self.write_text(&PdfText::new(text).font(font.clone()).color(*color), x, y)?;
        self.text_cursor.1 -= font.size + 4.0;
        Ok(self)
    }

    /// Add image from file path (hutool addPicture pattern).
    pub fn add_image_from_path(
        &mut self,
        path: impl AsRef<Path>,
        w_pt: f64,
        h_pt: f64,
    ) -> Result<&mut Self> {
        let img = PdfImage::from_path(path)?;
        let (x, y) = self.text_cursor;
        self.write_image(&img, x, y - h_pt, w_pt, h_pt)?;
        self.text_cursor.1 -= h_pt + 8.0;
        Ok(self)
    }

    /// Write the document to a file using atomic output with fsync.
    ///
    /// Finalizes the current page, fires `after_document` on all handlers,
    /// collects any spilled pages, constructs the final PDF, and writes it
    /// atomically (temp-file + fsync + rename).
    pub fn finish(mut self, path: impl AsRef<Path>) -> Result<()> {
        if self.current_page_number == 0 {
            self.add_page(PageSize::A4, Orientation::Portrait)?;
        }
        self.finalize_current_page()?;
        self.chain.after_document()?;

        // Apply easypdf metadata to printpdf document info before saving.
        // This ensures PdfMetadata set via builder methods is written into
        // the PDF's /Info dictionary, overriding the default title from
        // PdfDocument::new().
        self.apply_metadata();

        // Collect spilled pages (if any) and merge with in-memory pages.
        let mut all_pages = std::mem::take(&mut self.pages);
        if let Some(ref spill) = self.spill_writer {
            let spilled = spill.collect_all()?;
            for data in spilled {
                all_pages.push(PdfPage::new(
                    Mm(data.width_pt as f32 * PT_TO_MM as f32),
                    Mm(data.height_pt as f32 * PT_TO_MM as f32),
                    data.ops,
                ));
            }
        }

        self.doc.with_pages(all_pages);
        let opts = PdfSaveOptions::default();
        let mut warnings = Vec::new();
        let bytes = self.doc.save(&opts, &mut warnings);
        AtomicFileOutput::new(path.as_ref()).write_with_fsync(&bytes)
    }

    /// Copy easypdf metadata fields into the printpdf document info.
    fn apply_metadata(&mut self) {
        let info = &mut self.doc.metadata.info;
        if let Some(ref title) = self.metadata.title {
            info.document_title.clone_from(title);
        }
        if let Some(ref author) = self.metadata.author {
            info.author.clone_from(author);
        }
        if let Some(ref subject) = self.metadata.subject {
            info.subject.clone_from(subject);
        }
        if let Some(ref keywords) = self.metadata.keywords {
            info.keywords = keywords.split(',').map(|s| s.trim().to_string()).collect();
        }
        if let Some(ref creator) = self.metadata.creator {
            info.creator.clone_from(creator);
        }
        if let Some(ref producer) = self.metadata.producer {
            info.producer.clone_from(producer);
        }
    }

    /// Flush to the pre-configured output stream (hutool pattern).
    #[allow(clippy::similar_names)]
    pub fn flush(&mut self) -> Result<()> {
        let mut pages = std::mem::take(&mut self.pages);
        let ops = std::mem::take(&mut self.current_page_ops);
        if !ops.is_empty() {
            let (w, h) = self.current_page_size;
            pages.push(PdfPage::new(
                Mm(w as f32 * PT_TO_MM as f32),
                Mm(h as f32 * PT_TO_MM as f32),
                ops,
            ));
        }
        if pages.is_empty() {
            let (w, h) = self.current_page_size;
            pages.push(PdfPage::new(
                Mm(w as f32 * PT_TO_MM as f32),
                Mm(h as f32 * PT_TO_MM as f32),
                Vec::new(),
            ));
        }
        self.apply_metadata();
        self.doc.with_pages(pages);
        let opts = PdfSaveOptions::default();
        let mut warnings = Vec::new();
        if let Some(ref mut w) = self.output {
            self.doc.save_writer(w, &opts, &mut warnings);
        }
        Ok(())
    }
}

impl LayoutSink for PdfWriter {
    fn add_page(&mut self, size: PageSize, orientation: Orientation) -> Result<usize> {
        Self::add_page(self, size, orientation)
    }

    fn write_text(&mut self, text: &PdfText, x: f64, y: f64) -> Result<()> {
        Self::write_text(self, text, x, y)
    }

    fn finish(self, path: &Path) -> Result<()> {
        Self::finish(self, path)
    }
}
