//! Main PdfWriter struct and core PDF writing methods.

use easypdf_core::error::{PdfError, Result};
use easypdf_core::{
    FontFamily, Orientation, PageSize, PdfColor, PdfFont, PdfImage, PdfMetadata, PdfText,
    PdfWriteHandler,
};
use easypdf_io::AtomicFileOutput;
use easypdf_layout::LayoutSink;
use printpdf::{Mm, Op, PdfDocument, PdfFontHandle, PdfPage, PdfSaveOptions, Point, Pt, TextItem};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

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
/// Inspired by hutool's OfdWriter and ExcelWriter patterns.
pub struct PdfWriter {
    pub(crate) doc: PdfDocument,
    /// Accumulated completed pages.
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
    /// Lifecycle handlers.
    handlers: Vec<Box<dyn PdfWriteHandler>>,
    /// Auto-cursor for add_text convenience.
    text_cursor: (f64, f64),
    /// Output stream for flush-based writing.
    output: Option<Box<dyn Write>>,
}

impl PdfWriter {
    /// Create a new PDF document (writes to file via `finish`).
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
            handlers: Vec::new(),
            text_cursor: (DEFAULT_MARGIN, 0.0),
            output: None,
        }
    }

    /// Create a new PDF document that writes to a generic writer (hutool pattern).
    #[must_use]
    pub fn new_from_writer(writer: impl Write + 'static) -> Self {
        let mut s = Self::new("untitled");
        s.output = Some(Box::new(writer));
        s
    }

    /// Set document metadata.
    #[must_use]
    pub fn metadata(mut self, metadata: PdfMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Register a write handler for lifecycle callbacks.
    #[must_use]
    pub fn register_handler(mut self, handler: Box<dyn PdfWriteHandler>) -> Self {
        self.handlers.push(handler);
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
        &mut self, text: &str, font_key: &str, font_size: f64, x_pt: f64, y_pt: f64,
    ) -> Result<()> {
        let font_id = self.custom_fonts.get(font_key).cloned().ok_or_else(|| {
            PdfError::UnsupportedFeature(format!("Custom font '{font_key}' not registered."))
        })?;
        let pos = Point { x: Pt(x_pt as f32), y: Pt(y_pt as f32) };
        let ops = vec![
            Op::StartTextSection, Op::SetTextCursor { pos },
            Op::SetFont { font: PdfFontHandle::External(font_id), size: Pt(font_size as f32) },
            Op::ShowText { items: vec![TextItem::Text(text.to_string())] },
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
        for handler in &mut self.handlers {
            handler.before_page(self.current_page_number)?;
        }
        self.current_page_open = true;
        Ok(self.current_page_number)
    }

    fn finalize_current_page(&mut self) -> Result<()> {
        if !self.current_page_open {
            return Ok(());
        }
        for handler in &mut self.handlers {
            handler.after_page(self.current_page_number)?;
        }
        let ops = std::mem::take(&mut self.current_page_ops);
        let (w, h) = self.current_page_size;
        self.pages.push(PdfPage::new(Mm(w as f32 * PT_TO_MM as f32), Mm(h as f32 * PT_TO_MM as f32), ops));
        self.current_page_open = false;
        Ok(())
    }

    fn ensure_document_started(&mut self) -> Result<()> {
        if self.document_started {
            return Ok(());
        }
        for handler in &mut self.handlers {
            handler.before_document()?;
        }
        self.document_started = true;
        Ok(())
    }

    /// Get current page number (1-based).
    #[must_use] pub const fn current_page_number(&self) -> usize { self.current_page_number }
    /// Get total finalized pages.
    #[must_use] pub fn page_count(&self) -> usize { self.pages.len() }

    /// Write text at (x, y) in PDF points.
    pub fn write_text(&mut self, text: &PdfText, x_pt: f64, y_pt: f64) -> Result<()> {
        if let FontFamily::Custom(ref path) = text.font.family {
            if let Some(font_id) = self.custom_fonts.get(path.as_ref()) {
                let pos = Point { x: Pt(x_pt as f32), y: Pt(y_pt as f32) };
                let ops = vec![
                    Op::StartTextSection, Op::SetTextCursor { pos },
                    Op::SetFont { font: PdfFontHandle::External(font_id.clone()), size: Pt(text.font.size as f32) },
                    Op::ShowText { items: vec![TextItem::Text(text.content.clone())] },
                    Op::EndTextSection,
                ];
                self.current_page_ops.extend(ops);
                return Ok(());
            }
        }
        let bf = map_builtin_font(&text.font);
        let pos = Point { x: Pt(x_pt as f32), y: Pt(y_pt as f32) };
        let ops = vec![
            Op::StartTextSection, Op::SetTextCursor { pos },
            Op::SetFont { font: PdfFontHandle::Builtin(bf), size: Pt(text.font.size as f32) },
            Op::ShowText { items: vec![TextItem::Text(text.content.clone())] },
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
    pub fn add_text_colored(&mut self, font: &PdfFont, color: &PdfColor, text: &str) -> Result<&mut Self> {
        let (x, y) = self.text_cursor;
        self.write_text(&PdfText::new(text).font(font.clone()).color(*color), x, y)?;
        self.text_cursor.1 -= font.size + 4.0;
        Ok(self)
    }

    /// Add image from file path (hutool addPicture pattern).
    pub fn add_image_from_path(&mut self, path: impl AsRef<Path>, w_pt: f64, h_pt: f64) -> Result<&mut Self> {
        let img = PdfImage::from_path(path)?;
        let (x, y) = self.text_cursor;
        self.write_image(&img, x, y - h_pt, w_pt, h_pt)?;
        self.text_cursor.1 -= h_pt + 8.0;
        Ok(self)
    }

    /// Write the document to a file.
    pub fn finish(mut self, path: impl AsRef<Path>) -> Result<()> {
        if self.current_page_number == 0 {
            self.add_page(PageSize::A4, Orientation::Portrait)?;
        }
        self.finalize_current_page()?;
        for handler in &mut self.handlers {
            handler.after_document()?;
        }
        self.doc.with_pages(self.pages);
        let opts = PdfSaveOptions::default();
        let mut warnings = Vec::new();
        let bytes = self.doc.save(&opts, &mut warnings);
        AtomicFileOutput::new(path.as_ref()).write(&bytes)
    }

    /// Flush to the pre-configured output stream (hutool pattern).
    pub fn flush(&mut self) -> Result<()> {
        let mut pages = std::mem::take(&mut self.pages);
        let ops = std::mem::take(&mut self.current_page_ops);
        if !ops.is_empty() {
            let (w, h) = self.current_page_size;
            pages.push(PdfPage::new(Mm(w as f32 * PT_TO_MM as f32), Mm(h as f32 * PT_TO_MM as f32), ops));
        }
        if pages.is_empty() {
            let (w, h) = self.current_page_size;
            pages.push(PdfPage::new(Mm(w as f32 * PT_TO_MM as f32), Mm(h as f32 * PT_TO_MM as f32), Vec::new()));
        }
        self.doc.with_pages(pages);
        let opts = PdfSaveOptions::default();
        let mut warnings = Vec::new();
        if let Some(ref mut w) = self.output { self.doc.save_writer(w, &opts, &mut warnings); }
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
