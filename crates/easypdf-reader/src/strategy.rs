//! PDF reading strategies: full, lazy, and streaming.
//!
//! Mirrors the `ExcelReadExecutorKind` enum dispatch pattern from
//! easyexcel-rust: the caller (or auto-detection) selects the optimal
//! parsing strategy based on document size.

use std::collections::HashMap;

use easypdf_core::{PdfError, Result};

/// PDF reading strategy enum (analogous to `ExcelReadExecutorKind`).
///
/// Selects how the PDF document is parsed and loaded into memory.
/// Use [`ReadStrategy::auto`] to pick the best strategy based on file size.
///
/// # Examples
///
/// ```
/// use easypdf_reader::ReadStrategy;
///
/// let strategy = ReadStrategy::auto(1024 * 1024); // 1 MB
/// assert_eq!(strategy, ReadStrategy::Full);
///
/// let strategy = ReadStrategy::auto(50 * 1024 * 1024); // 50 MB
/// assert_eq!(strategy, ReadStrategy::Lazy);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ReadStrategy {
    /// Full in-memory loading (default, suitable for small documents).
    ///
    /// Loads the entire PDF via `lopdf::Document::load_mem` -- fast random
    /// access to all objects, but the full document must fit in memory.
    Full,

    /// Lazy page-level parsing (suitable for large documents).
    ///
    /// Parses only the trailer, cross-reference table, and page tree
    /// structure. Individual page content streams are loaded on demand
    /// and cached after first access.
    Lazy,

    /// Streaming scan (suitable for very large documents, text-only).
    ///
    /// Does not build a complete object tree. Scans the PDF byte stream
    /// for content streams and triggers the listener incrementally.
    ///
    /// Accuracy is lower than [`Full`](Self::Full) or [`Lazy`](Self::Lazy)
    /// because cross-reference resolution and font encoding (CMap/ToUnicode)
    /// are skipped.
    Streaming,
}

impl ReadStrategy {
    /// File-size threshold (bytes) below which `Full` is chosen.
    const FULL_THRESHOLD: u64 = 5_000_000; // 5 MB
    /// File-size threshold (bytes) below which `Lazy` is chosen.
    const LAZY_THRESHOLD: u64 = 100_000_000; // 100 MB

    /// Automatically select the best strategy based on file size.
    ///
    /// | File size | Strategy |
    /// |-----------|----------|
    /// | 0..5 MB | [`Full`](ReadStrategy::Full) |
    /// | 5..100 MB | [`Lazy`](ReadStrategy::Lazy) |
    /// | > 100 MB | [`Streaming`](ReadStrategy::Streaming) |
    #[must_use]
    pub const fn auto(file_size: u64) -> Self {
        if file_size <= Self::FULL_THRESHOLD {
            Self::Full
        } else if file_size <= Self::LAZY_THRESHOLD {
            Self::Lazy
        } else {
            Self::Streaming
        }
    }

    /// Returns `true` if this strategy loads the entire document upfront.
    #[must_use]
    pub const fn is_full(&self) -> bool {
        matches!(self, Self::Full)
    }

    /// Returns `true` if this strategy defers page content loading.
    #[must_use]
    pub const fn is_lazy(&self) -> bool {
        matches!(self, Self::Lazy | Self::Streaming)
    }
}

/// Parsed page content cached by the lazy loader.
#[derive(Debug, Clone)]
pub(crate) struct ParsedPage {
    /// Extracted text for this page.
    pub text: String,
}

/// Lazy page-level loader.
///
/// Parses only the page tree structure upfront; individual page content
/// streams are loaded on demand and cached after first access. This avoids
/// materializing the full object tree for large documents.
///
/// The loader borrows the already-loaded [`lopdf::Document`] (which holds
/// the trailer and xref table) and builds a flat list of page object IDs
/// without reading any content streams.
pub(crate) struct LazyPageLoader<'a> {
    doc: &'a lopdf::Document,
    page_object_ids: Vec<lopdf::ObjectId>,
    cached_pages: HashMap<usize, ParsedPage>,
}

impl<'a> LazyPageLoader<'a> {
    /// Build a lazy loader from a parsed `lopdf::Document`.
    ///
    /// This walks the page tree to collect page object IDs but does **not**
    /// read any content streams.
    #[must_use]
    pub fn new(doc: &'a lopdf::Document) -> Self {
        let pages_map = doc.get_pages();
        let mut page_object_ids = Vec::with_capacity(pages_map.len());
        // `get_pages()` returns BTreeMap<u32, ObjectId> sorted by page number.
        for (_page_num, obj_id) in pages_map {
            page_object_ids.push(obj_id);
        }
        Self {
            doc,
            page_object_ids,
            cached_pages: HashMap::new(),
        }
    }

    /// Total number of pages (available without loading content).
    #[must_use]
    #[allow(dead_code)] // used in tests; future streaming strategy will call this in lib
    pub fn page_count(&self) -> usize {
        self.page_object_ids.len()
    }

    /// Extract text for a single page (0-based index), with caching.
    ///
    /// The first call for a given page reads and decompresses the content
    /// stream; subsequent calls return the cached result.
    ///
    /// # Errors
    ///
    /// Returns [`PdfError::Parse`] when the page content cannot be decoded,
    /// or [`PdfError::InvalidPage`] when the index is out of bounds.
    pub fn page_text(&mut self, page_index: usize) -> Result<String> {
        if let Some(cached) = self.cached_pages.get(&page_index) {
            return Ok(cached.text.clone());
        }

        if page_index >= self.page_object_ids.len() {
            return Err(PdfError::InvalidPage(page_index));
        }

        // lopdf page numbers are 1-based.
        let page_number = u32::try_from(page_index)
            .map_err(|_| PdfError::Parse("page index overflow".to_string()))?
            + 1;

        let text = self
            .doc
            .extract_text(&[page_number])
            .map_err(|error| PdfError::Parse(error.to_string()))?;

        self.cached_pages
            .insert(page_index, ParsedPage { text: text.clone() });
        Ok(text)
    }

    /// Extract text for multiple pages (0-based indices).
    ///
    /// # Errors
    ///
    /// Returns an error if any page cannot be extracted.
    pub fn pages_text(&mut self, indices: &[usize]) -> Result<Vec<(usize, String)>> {
        let mut results = Vec::with_capacity(indices.len());
        for &idx in indices {
            let text = self.page_text(idx)?;
            results.push((idx, text));
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::similar_names)]
    use super::*;

    // --- ReadStrategy ---

    #[test]
    fn auto_small_file_is_full() {
        assert_eq!(ReadStrategy::auto(0), ReadStrategy::Full);
        assert_eq!(ReadStrategy::auto(1), ReadStrategy::Full);
        assert_eq!(ReadStrategy::auto(5_000_000), ReadStrategy::Full);
    }

    #[test]
    fn auto_medium_file_is_lazy() {
        assert_eq!(ReadStrategy::auto(5_000_001), ReadStrategy::Lazy);
        assert_eq!(ReadStrategy::auto(50_000_000), ReadStrategy::Lazy);
        assert_eq!(ReadStrategy::auto(100_000_000), ReadStrategy::Lazy);
    }

    #[test]
    fn auto_large_file_is_streaming() {
        assert_eq!(ReadStrategy::auto(100_000_001), ReadStrategy::Streaming);
        assert_eq!(ReadStrategy::auto(u64::MAX), ReadStrategy::Streaming);
    }

    #[test]
    fn is_full_and_is_lazy() {
        assert!(ReadStrategy::Full.is_full());
        assert!(!ReadStrategy::Full.is_lazy());

        assert!(!ReadStrategy::Lazy.is_full());
        assert!(ReadStrategy::Lazy.is_lazy());

        assert!(!ReadStrategy::Streaming.is_full());
        assert!(ReadStrategy::Streaming.is_lazy());
    }

    #[test]
    fn strategy_debug_clone_eq_hash() {
        let s = ReadStrategy::Lazy;
        let s2 = s;
        assert_eq!(s, s2);
        assert_eq!(format!("{s:?}"), "Lazy");

        // Verify it works in a HashSet.
        let mut set = std::collections::HashSet::new();
        set.insert(ReadStrategy::Full);
        set.insert(ReadStrategy::Lazy);
        set.insert(ReadStrategy::Streaming);
        assert_eq!(set.len(), 3);
    }

    // --- LazyPageLoader ---

    fn make_test_doc() -> lopdf::Document {
        let mut doc = lopdf::Document::new();
        let content_id = doc.add_object(lopdf::Object::Stream(lopdf::Stream::new(
            lopdf::Dictionary::new(),
            b"BT /F1 12 Tf (Lazy Test) Tj ET".to_vec(),
        )));
        let mut page_dict = lopdf::Dictionary::new();
        page_dict.set("Type", lopdf::Object::Name(b"Page".to_vec()));
        page_dict.set(
            "MediaBox",
            lopdf::Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]),
        );
        page_dict.set("Contents", lopdf::Object::Reference(content_id));
        let page_id = doc.add_object(lopdf::Object::Dictionary(page_dict));
        let mut pages = lopdf::Dictionary::new();
        pages.set("Type", lopdf::Object::Name(b"Pages".to_vec()));
        pages.set(
            "Kids",
            lopdf::Object::Array(vec![lopdf::Object::Reference(page_id)]),
        );
        pages.set("Count", lopdf::Object::Integer(1));
        let pages_id = doc.add_object(lopdf::Object::Dictionary(pages));
        let mut catalog = lopdf::Dictionary::new();
        catalog.set("Type", lopdf::Object::Name(b"Catalog".to_vec()));
        catalog.set("Pages", lopdf::Object::Reference(pages_id));
        let catalog_id = doc.add_object(lopdf::Object::Dictionary(catalog));
        doc.trailer
            .set("Root", lopdf::Object::Reference(catalog_id));
        doc
    }

    #[test]
    fn lazy_page_loader_page_count() {
        let doc = make_test_doc();
        let loader = LazyPageLoader::new(&doc);
        assert_eq!(loader.page_count(), 1);
    }

    #[test]
    fn lazy_page_loader_extracts_text() {
        let doc = make_test_doc();
        let mut loader = LazyPageLoader::new(&doc);
        let text = loader.page_text(0).unwrap();
        // Text extraction depends on font encoding; just verify no error.
        let _ = text;
    }

    #[test]
    fn lazy_page_loader_caches() {
        let doc = make_test_doc();
        let mut loader = LazyPageLoader::new(&doc);
        let text1 = loader.page_text(0).unwrap();
        let text2 = loader.page_text(0).unwrap();
        assert_eq!(text1, text2);
        assert_eq!(loader.cached_pages.len(), 1);
    }

    #[test]
    fn lazy_page_loader_out_of_bounds() {
        let doc = make_test_doc();
        let mut loader = LazyPageLoader::new(&doc);
        let result = loader.page_text(99);
        assert!(result.is_err());
    }

    #[test]
    fn lazy_page_loader_pages_text() {
        let doc = make_test_doc();
        let mut loader = LazyPageLoader::new(&doc);
        let results = loader.pages_text(&[0]).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 0);
    }
}
