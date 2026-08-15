//! Text, metadata, and document model extraction for [`PdfReader`].

use easypdf_core::error::{PdfError, Result};
use easypdf_core::{PageIndex, PdfMetadata, PdfReadListener};
use easypdf_core::{PdfBlock, PdfDocumentModel, PdfPageModel, SourceLocation};

use crate::strategy::{LazyPageLoader, ReadStrategy};
use crate::streaming::StreamScanner;

use super::PdfReader;
use super::usize_to_u64_saturating;

impl PdfReader {
    /// Extract text from all selected pages, joined with newlines.
    ///
    /// For [`ReadStrategy::Streaming`], this scans the raw bytes directly
    /// without building a `lopdf::Document` object tree.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Parse` if the PDF content cannot be read.
    pub fn extract_text(&self) -> Result<String> {
        if self.strategy == ReadStrategy::Streaming {
            return self.extract_text_streaming();
        }

        let mut all_text = String::new();
        for (_, page_number) in self.selected_pages() {
            let text = self.extract_page_text(page_number)?;
            if !all_text.is_empty() {
                all_text.push('\n');
            }
            self.ensure_text_limit(all_text.len() + text.len())?;
            all_text.push_str(&text);
        }
        Ok(all_text)
    }

    /// Extract metadata from the PDF document.
    ///
    /// For [`ReadStrategy::Streaming`], this performs a heuristic scan of the
    /// raw bytes (no xref resolution).  For other strategies, it reads the
    /// `/Info` dictionary via the parsed `lopdf::Document`.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Parse` if the document cannot be read.
    ///
    /// # Panics
    ///
    /// Panics if called on a non-Streaming reader whose `document` is `None`
    /// (should never happen in normal usage).
    pub fn extract_metadata(&self) -> Result<PdfMetadata> {
        if self.strategy == ReadStrategy::Streaming {
            let scanner = StreamScanner::new(&self.raw_bytes, self.limits);
            return Ok(scanner.extract_metadata_quick());
        }

        // Non-Streaming: read /Info dictionary from the parsed document.
        let doc = self
            .document
            .as_ref()
            .expect("document must be Some for non-Streaming strategies");

        let info_dict = doc
            .trailer
            .get(b"Info")
            .ok()
            .and_then(|info| {
                let info_id = info.as_reference().ok()?;
                doc.get_object(info_id).ok()
            })
            .and_then(|obj| obj.as_dict().ok());

        let title = info_dict.as_ref().and_then(|dict| {
            dict.get(b"Title")
                .ok()
                .and_then(|v| v.as_str().ok())
                .map(decode_pdf_string)
        });

        let author = info_dict.as_ref().and_then(|dict| {
            dict.get(b"Author")
                .ok()
                .and_then(|v| v.as_str().ok())
                .map(decode_pdf_string)
        });

        Ok(PdfMetadata {
            title,
            author,
            subject: None,
            keywords: None,
            creator: None,
            producer: None,
        })
    }

    /// Get the total number of pages in the document.
    ///
    /// For [`ReadStrategy::Streaming`], this returns a heuristic count based
    /// on `/Type /Page` entries found in the raw bytes.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Parse` if the document cannot be read.
    ///
    /// # Panics
    ///
    /// Panics if called on a non-Streaming reader whose `document` is `None`
    /// (should never happen in normal usage).
    pub fn page_count(&self) -> Result<usize> {
        if self.strategy == ReadStrategy::Streaming {
            let scanner = StreamScanner::new(&self.raw_bytes, self.limits);
            return Ok(scanner.page_count());
        }
        Ok(self
            .document
            .as_ref()
            .expect("document must be Some for non-Streaming strategies")
            .get_pages()
            .len())
    }

    /// Extract an engine-neutral semantic document model.
    ///
    /// The initial reader backend emits paragraph blocks. Higher-level analyzers can
    /// later enrich these blocks with headings, tables, images, and OCR results.
    ///
    /// # Errors
    ///
    /// Returns an error when text extraction fails or a resource limit is exceeded.
    pub fn extract_document_model(&self) -> Result<PdfDocumentModel> {
        if self.strategy == ReadStrategy::Streaming {
            let text = self.extract_text_streaming()?;
            let source = SourceLocation::new(PageIndex::new(0), 1.0);
            let mut page = PdfPageModel::new(PageIndex::new(0));
            for paragraph in split_paragraphs(&text) {
                page = page.with_block(PdfBlock::paragraph(paragraph, source));
            }
            return Ok(PdfDocumentModel::new(self.extract_metadata()?, vec![page]));
        }

        let mut pages = Vec::new();
        let mut extracted_bytes = 0usize;
        for (page_index, page_number) in self.selected_pages() {
            let text = self.extract_page_text(page_number)?;
            extracted_bytes = extracted_bytes.saturating_add(text.len());
            self.ensure_text_limit(extracted_bytes)?;
            let source = SourceLocation::new(PageIndex::new(page_index), 1.0);
            let mut page = PdfPageModel::new(PageIndex::new(page_index));
            for paragraph in split_paragraphs(&text) {
                page = page.with_block(PdfBlock::paragraph(paragraph, source));
            }
            pages.push(page);
        }
        Ok(PdfDocumentModel::new(self.extract_metadata()?, pages))
    }

    /// Read the document with an event-driven listener (dynamic dispatch).
    ///
    /// This method uses `&mut dyn PdfReadListener` for dynamic dispatch.
    /// For zero-cost monomorphization, use [`Self::read_with_listener_typed`].
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Parse` if the document cannot be read.
    pub fn read_with_listener(&self, listener: &mut dyn PdfReadListener) -> Result<()> {
        if self.strategy == ReadStrategy::Streaming {
            let scanner = StreamScanner::new(&self.raw_bytes, self.limits);
            scanner.scan(listener)?;
            return Ok(());
        }

        for (page_index, page_number) in self.selected_pages() {
            let displayed_page = page_index + 1;
            let page_text = self.extract_page_text(page_number)?;
            listener.on_page_start(displayed_page)?;
            if !page_text.is_empty() {
                listener.on_text(displayed_page, &page_text)?;
            }
            listener.on_page_end(displayed_page)?;
        }
        listener.on_document_end()?;
        Ok(())
    }

    /// Read the document with a typed event-driven listener (static dispatch).
    ///
    /// This is the monomorphized version of [`read_with_listener`](Self::read_with_listener).
    /// The compiler generates a specialized version for each concrete listener type,
    /// eliminating vtable overhead.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Parse` if the document cannot be read.
    pub fn read_with_listener_typed<L: PdfReadListener>(&self, listener: &mut L) -> Result<()> {
        if self.strategy == ReadStrategy::Streaming {
            let scanner = StreamScanner::new(&self.raw_bytes, self.limits);
            scanner.scan(listener)?;
            return Ok(());
        }

        for (page_index, page_number) in self.selected_pages() {
            let displayed_page = page_index + 1;
            let page_text = self.extract_page_text(page_number)?;
            listener.on_page_start(displayed_page)?;
            if !page_text.is_empty() {
                listener.on_text(displayed_page, &page_text)?;
            }
            listener.on_page_end(displayed_page)?;
        }
        listener.on_document_end()?;
        Ok(())
    }

    /// Extract text using the lazy loading strategy.
    ///
    /// When the reader was opened with [`ReadStrategy::Lazy`], this method
    /// uses a lazy page loader to load page content on demand with caching.
    /// For [`ReadStrategy::Full`], it delegates to [`extract_text`](Self::extract_text).
    /// For [`ReadStrategy::Streaming`], it delegates to the streaming scanner.
    ///
    /// # Errors
    ///
    /// Returns an error when page content cannot be decoded.
    ///
    /// # Panics
    ///
    /// Panics if called on a non-Streaming reader whose `document` is `None`
    /// (should never happen in normal usage).
    pub fn extract_text_lazy(&mut self) -> Result<String> {
        if self.strategy == ReadStrategy::Streaming {
            return self.extract_text_streaming();
        }

        if self.strategy.is_full() {
            return self.extract_text();
        }

        let doc = self
            .document
            .as_ref()
            .expect("document must be Some for non-Streaming strategies");
        let mut loader = LazyPageLoader::new(doc);
        let mut all_text = String::new();
        let indices: Vec<usize> = self.selected_pages().map(|(index, _)| index).collect();

        for (idx, text) in loader.pages_text(&indices)? {
            if !all_text.is_empty() {
                all_text.push('\n');
            }
            self.ensure_text_limit(all_text.len() + text.len())?;
            all_text.push_str(&text);
            let _ = idx;
        }
        Ok(all_text)
    }

    // --- Private helpers ---

    /// Streaming text extraction via `StreamScanner`.
    pub(super) fn extract_text_streaming(&self) -> Result<String> {
        struct TextCollector {
            parts: Vec<String>,
        }
        impl PdfReadListener for TextCollector {
            fn on_text(&mut self, _page: usize, text: &str) -> easypdf_core::Result<()> {
                self.parts.push(text.to_string());
                Ok(())
            }
        }

        let scanner = StreamScanner::new(&self.raw_bytes, self.limits);
        let mut collector = TextCollector { parts: vec![] };
        scanner.scan(&mut collector)?;

        let mut all_text = String::new();
        for part in &collector.parts {
            if !all_text.is_empty() {
                all_text.push('\n');
            }
            self.ensure_text_limit(all_text.len() + part.len())?;
            all_text.push_str(part);
        }
        Ok(all_text)
    }

    /// Return selected pages for non-Streaming strategies.
    ///
    /// # Panics
    ///
    /// Panics if `self.document` is `None` (Streaming strategy).
    pub(super) fn selected_pages(&self) -> impl Iterator<Item = (usize, u32)> + '_ {
        self.document
            .as_ref()
            .expect("selected_pages called for Streaming strategy; use StreamScanner instead")
            .get_pages()
            .into_keys()
            .enumerate()
            .filter(|(index, _)| {
                self.pages
                    .as_ref()
                    .is_none_or(|range| range.contains(*index))
            })
    }

    /// Extract text from a single page (non-Streaming strategies).
    ///
    /// # Panics
    ///
    /// Panics if `self.document` is `None` (Streaming strategy).
    pub(super) fn extract_page_text(&self, page_number: u32) -> Result<String> {
        self.document
            .as_ref()
            .expect("extract_page_text called for Streaming strategy")
            .extract_text(&[page_number])
            .map_err(|error| PdfError::Parse(error.to_string()))
    }

    pub(super) fn ensure_text_limit(&self, bytes: usize) -> Result<()> {
        if bytes > self.limits.max_extracted_text_bytes() {
            return Err(PdfError::ResourceLimitExceeded {
                resource: "extracted_text_bytes",
                limit: usize_to_u64_saturating(self.limits.max_extracted_text_bytes()),
                actual: usize_to_u64_saturating(bytes),
            });
        }
        Ok(())
    }
}

/// Split text into paragraphs on double newlines.
fn split_paragraphs(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split("\n\n")
        .map(str::trim)
        .filter(|paragraph| !paragraph.is_empty())
        .map(ToOwned::to_owned)
}

/// Decode a PDF string object to a Rust `String`.
///
/// PDF strings in the `/Info` dictionary may be encoded as:
/// - **UTF-16BE** with a BOM (`\xFE\xFF`) prefix -- used by printpdf and
///   most modern PDF producers for non-ASCII or even all text.
/// - **`PDFDocEncoding`** (a superset of Latin-1) -- the default when no BOM
///   is present.
///
/// This function checks for the UTF-16BE BOM and decodes accordingly,
/// falling back to UTF-8 lossy decoding for `PDFDocEncoding` / Latin-1 bytes.
fn decode_pdf_string(bytes: &[u8]) -> String {
    // Check for UTF-16BE BOM (0xFE 0xFF).
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let utf16: Vec<u16> = bytes[2..]
            .chunks(2)
            .map(|chunk| {
                if chunk.len() == 2 {
                    u16::from_be_bytes([chunk[0], chunk[1]])
                } else {
                    // Trailing single byte -- replace with U+FFFD.
                    0xFFFD
                }
            })
            .collect();
        return String::from_utf16_lossy(&utf16);
    }
    // No BOM: treat as PDFDocEncoding / Latin-1 / UTF-8.
    // from_utf8_lossy handles invalid bytes with U+FFFD replacement.
    String::from_utf8_lossy(bytes).into_owned()
}
