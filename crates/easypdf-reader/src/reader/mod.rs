//! PDF reading and text extraction (lopdf backend).
//!
//! Provides [`PdfReader`] for parsing PDF documents and extracting text,
//! metadata, and page information.

mod extract;

#[cfg(test)]
#[allow(clippy::items_after_statements, clippy::similar_names)]
mod tests;

use std::ops::Range;
use std::path::Path;

use easypdf_core::error::{PdfError, Result};
use easypdf_core::io::guards::guard_element_explosion;
use easypdf_core::io::repair::{attempt_repair, is_likely_corrupt, RepairOptions};
use easypdf_core::{PageRange, PdfInput, ResourceLimits};

use super::strategy::ReadStrategy;

/// A reader for extracting content from PDF documents.
///
/// Backed by the `lopdf` crate for low-level PDF parsing. Supports multiple
/// reading strategies ([`ReadStrategy`]) for optimal performance across
/// document sizes.
///
/// # Examples
///
/// ```no_run
/// use easypdf_reader::PdfReader;
///
/// let text = PdfReader::open("document.pdf")?.extract_text()?;
/// # Ok::<(), easypdf_core::PdfError>(())
/// ```
pub struct PdfReader {
    /// Parsed document (`None` for [`ReadStrategy::Streaming`]).
    pub(super) document: Option<lopdf::Document>,
    pub(super) pages: Option<PageRange>,
    pub(super) limits: ResourceLimits,
    pub(super) strategy: ReadStrategy,
    /// Raw PDF bytes -- retained for Streaming strategy.
    pub(super) raw_bytes: Vec<u8>,
}

impl PdfReader {
    /// Open a PDF file for reading with automatic strategy selection.
    ///
    /// The file size determines the [`ReadStrategy`]:
    /// files under 5 MB use [`Full`](ReadStrategy::Full),
    /// 5--100 MB use [`Lazy`](ReadStrategy::Lazy),
    /// and larger files use [`Streaming`](ReadStrategy::Streaming).
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Parse` if the file cannot be opened or is not a valid PDF.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let file_size = std::fs::metadata(path)
            .map_or(0, |m| m.len());
        let strategy = ReadStrategy::auto(file_size);
        Self::open_with_strategy(path, strategy)
    }

    /// Open a PDF from in-memory bytes with automatic strategy selection.
    ///
    /// # Errors
    ///
    /// Returns [`PdfError::Parse`] when the bytes are not a valid PDF.
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Result<Self> {
        let bytes = bytes.into();
        let file_size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        let strategy = ReadStrategy::auto(file_size);
        let input = PdfInput::from_bytes(bytes);
        Self::open_with_limits_and_strategy(&input, ResourceLimits::default(), strategy)
    }

    /// Open a PDF input with explicit resource limits.
    ///
    /// The document is parsed exactly once and retained by the reader session.
    ///
    /// # Errors
    ///
    /// Returns an error when input limits are exceeded or parsing fails.
    pub fn open_with_limits(input: &PdfInput, limits: ResourceLimits) -> Result<Self> {
        let bytes = input.read(limits)?;
        let file_size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        let strategy = ReadStrategy::auto(file_size);
        Self::load_from_bytes(bytes, limits, strategy)
    }

    /// Open a PDF file with an explicit [`ReadStrategy`].
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read, parsed, or exceeds
    /// resource limits.
    pub fn open_with_strategy(path: impl AsRef<Path>, strategy: ReadStrategy) -> Result<Self> {
        let input = PdfInput::from_path(path.as_ref());
        Self::open_with_limits_and_strategy(&input, ResourceLimits::default(), strategy)
    }

    /// Open a PDF file with explicit repair options and reading strategy.
    ///
    /// If [`is_likely_corrupt`] detects corruption, [`attempt_repair`] is
    /// called with the provided [`RepairOptions`] before loading.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read, repaired, parsed,
    /// or exceeds resource limits.
    pub fn open_with_repair(
        path: impl AsRef<Path>,
        repair: RepairOptions,
        strategy: ReadStrategy,
    ) -> Result<Self> {
        let input = PdfInput::from_path(path.as_ref());

        let bytes = if is_likely_corrupt(&input) {
            attempt_repair(&input, &repair)?
        } else {
            input.read(ResourceLimits::default())?
        };

        Self::load_from_bytes(bytes, ResourceLimits::default(), strategy)
    }

    /// Open a PDF input with explicit resource limits and reading strategy.
    ///
    /// # Errors
    ///
    /// Returns an error when input limits are exceeded or parsing fails.
    pub fn open_with_limits_and_strategy(
        input: &PdfInput,
        limits: ResourceLimits,
        strategy: ReadStrategy,
    ) -> Result<Self> {
        let bytes = input.read(limits)?;
        Self::load_from_bytes(bytes, limits, strategy)
    }

    /// Internal: load a PDF from raw bytes with the given limits and strategy.
    ///
    /// Applies security guards (element explosion) before parsing.  The
    /// `Streaming` strategy skips `lopdf::Document` construction entirely.
    fn load_from_bytes(
        bytes: Vec<u8>,
        limits: ResourceLimits,
        strategy: ReadStrategy,
    ) -> Result<Self> {
        if strategy == ReadStrategy::Streaming {
            // Streaming: no lopdf::Document needed -- scan raw bytes directly.
            return Ok(Self {
                document: None,
                pages: None,
                limits,
                strategy,
                raw_bytes: bytes,
            });
        }

        let document = lopdf::Document::load_mem(&bytes)
            .map_err(|error| PdfError::Parse(error.to_string()))?;

        // Guard: element explosion -- check total object count.
        let element_count = document.objects.len();
        guard_element_explosion(element_count, &limits)?;

        let page_count = document.get_pages().len();
        if page_count > limits.max_pages() {
            return Err(PdfError::ResourceLimitExceeded {
                resource: "pages",
                limit: usize_to_u64_saturating(limits.max_pages()),
                actual: usize_to_u64_saturating(page_count),
            });
        }

        Ok(Self {
            document: Some(document),
            pages: None,
            limits,
            strategy,
            raw_bytes: bytes,
        })
    }

    /// Returns the reading strategy this reader was opened with.
    #[must_use]
    pub const fn strategy(&self) -> ReadStrategy {
        self.strategy
    }

    /// Limit extraction to a specific page range (0-based).
    #[must_use]
    pub fn pages(mut self, range: Range<usize>) -> Self {
        let start = range.start;
        self.pages = Some(match PageRange::new(range) {
            Ok(pages) => pages,
            Err(_) => PageRange::empty_at(start),
        });
        self
    }

    /// Try to limit extraction to a validated zero-based page range.
    ///
    /// # Errors
    ///
    /// Returns an error when the range is inverted.
    pub fn try_pages(mut self, range: Range<usize>) -> Result<Self> {
        self.pages = Some(PageRange::new(range)?);
        Ok(self)
    }
}

/// Convert `usize` to `u64`, saturating at `u64::MAX` on overflow.
fn usize_to_u64_saturating(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
