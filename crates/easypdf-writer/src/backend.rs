//! Write backend selection and page-level spill mechanism.
//!
//! Provides [`WriteBackend`] for choosing between in-memory and spill-to-disk
//! modes, and [`PageSpillWriter`] for serializing finalized page content to
//! temporary files to bound peak memory usage.
//!
//! # Design
//!
//! The spill mechanism operates at page granularity: once a page is finalized,
//! its operations (`Vec<printpdf::Op>`) and dimensions are serialized to a
//! temporary file (optionally gzip-compressed). At `finish()` time, all spilled
//! pages are read back and merged into the final PDF document.
//!
//! This mirrors the SXSSF spill pattern from `easyexcel-rust`, adapted for
//! PDF's page-level (rather than row-level) content unit.

use easypdf_core::error::{PdfError, Result};
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use printpdf::Op;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write as _};
use std::path::PathBuf;

/// Serialized representation of a single page's content, stored in spill files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SpilledPageData {
    /// Page number (1-based).
    pub page_number: usize,
    /// Page width in points.
    pub width_pt: f64,
    /// Page height in points.
    pub height_pt: f64,
    /// The printpdf operations for this page.
    pub ops: Vec<Op>,
}

/// PDF write backend selection.
///
/// Controls whether the writer keeps all pages in memory or spills finalized
/// pages to temporary files to bound peak memory usage.
///
/// # Examples
///
/// ```
/// use easypdf_writer::WriteBackend;
///
/// // Auto-select based on expected page count.
/// let backend = WriteBackend::auto(50);
/// assert!(!backend.is_constant_memory());
///
/// let backend = WriteBackend::auto(200);
/// assert!(backend.is_constant_memory());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum WriteBackend {
    /// Full in-memory mode (default for small documents).
    ///
    /// The entire PDF document is constructed in memory via `printpdf`.
    /// Suitable for documents up to ~100 pages.
    #[default]
    InMemory,

    /// Page-level spill mode for large documents.
    ///
    /// Each finalized page is serialized to a temporary file and dropped from
    /// memory. At `finish()` time, all spilled pages are read back and merged.
    /// This bounds peak memory to approximately one page's content plus the
    /// final PDF output buffer.
    Spill {
        /// Directory for spill files. `None` uses the system temporary directory.
        spill_dir: Option<PathBuf>,
        /// Whether to gzip-compress spill files (reduces disk I/O at the cost
        /// of CPU, mirroring `compress_temp_files` from easyexcel-rust).
        compress: bool,
        /// Page count threshold: spill activates only after this many pages
        /// have been finalized. Pages below the threshold stay in memory.
        threshold_pages: usize,
    },
}

impl WriteBackend {
    /// Automatically select a backend based on the estimated page count.
    ///
    /// For 100 or fewer pages, returns [`InMemory`](Self::InMemory).
    /// For more than 100 pages, returns [`Spill`](Self::Spill) with compression
    /// enabled and a threshold of 50 pages.
    #[must_use]
    pub fn auto(estimated_pages: usize) -> Self {
        match estimated_pages {
            0..=100 => Self::InMemory,
            _ => Self::Spill {
                spill_dir: None,
                compress: true,
                threshold_pages: 50,
            },
        }
    }

    /// Returns `true` if this backend uses the constant-memory spill strategy.
    #[must_use]
    pub const fn is_constant_memory(&self) -> bool {
        matches!(self, Self::Spill { .. })
    }

    /// Create a spill backend with constant-memory semantics (threshold = 1).
    ///
    /// This is a convenience constructor that spills every page immediately
    /// after finalization.
    #[must_use]
    pub fn constant_memory() -> Self {
        Self::Spill {
            spill_dir: None,
            compress: true,
            threshold_pages: 1,
        }
    }
}

/// Page-level spill writer.
///
/// Manages serialization of finalized page content to temporary files and
/// deserialization at finish time. Spill files are stored in a temporary
/// directory that is automatically cleaned up when the `PageSpillWriter` is
/// dropped (via [`tempfile::TempDir`]).
pub(crate) struct PageSpillWriter {
    /// Temporary directory guard (cleaned up on drop).
    _temp_dir: Option<tempfile::TempDir>,
    /// Directory for spill files (either user-provided or from `_temp_dir`).
    spill_dir: PathBuf,
    /// Whether to gzip-compress spill files.
    compress: bool,
    /// Threshold: only spill when finalized page count exceeds this value.
    threshold_pages: usize,
    /// Paths of spilled page files, keyed by page number (1-based).
    spilled_pages: BTreeMap<usize, PathBuf>,
    /// Count of pages finalized so far (including both in-memory and spilled).
    finalized_count: usize,
}

impl PageSpillWriter {
    /// Create a new spill writer.
    ///
    /// If `spill_dir` is `None`, a unique temporary directory is created
    /// automatically (cleaned up on drop).
    ///
    /// # Errors
    ///
    /// Returns an error if the spill directory cannot be created.
    pub fn new(spill_dir: Option<PathBuf>, compress: bool, threshold_pages: usize) -> Result<Self> {
        let (dir, temp_dir_guard) = if let Some(d) = spill_dir {
            std::fs::create_dir_all(&d)?;
            (d, None)
        } else {
            let td = tempfile::tempdir().map_err(PdfError::Io)?;
            let path = td.path().to_path_buf();
            (path, Some(td))
        };
        Ok(Self {
            _temp_dir: temp_dir_guard,
            spill_dir: dir,
            compress,
            threshold_pages,
            spilled_pages: BTreeMap::new(),
            finalized_count: 0,
        })
    }

    /// Attempt to spill a finalized page's data.
    ///
    /// If the finalized page count has not yet exceeded the threshold, this is
    /// a no-op and the caller should keep the page data in memory.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or file I/O fails.
    pub fn maybe_spill(&mut self, page_data: &SpilledPageData) -> Result<Option<()>> {
        self.finalized_count += 1;
        if self.finalized_count <= self.threshold_pages {
            return Ok(None);
        }

        let file_name = format!(
            "page-{:06}.json{}",
            page_data.page_number,
            if self.compress { ".gz" } else { "" }
        );
        let file_path = self.spill_dir.join(&file_name);

        let serialized = serde_json::to_vec(page_data)
            .map_err(|e| PdfError::Other(format!("Spill serialization failed: {e}")))?;

        if self.compress {
            let file = File::create(&file_path)?;
            let mut encoder = GzEncoder::new(BufWriter::new(file), Compression::fast());
            encoder.write_all(&serialized)?;
            encoder.try_finish()?;
        } else {
            std::fs::write(&file_path, &serialized)?;
        }

        self.spilled_pages.insert(page_data.page_number, file_path);
        Ok(Some(()))
    }

    /// Collect all spilled pages, returning them in page-number order.
    ///
    /// # Errors
    ///
    /// Returns an error if any spill file cannot be read or deserialized.
    pub fn collect_all(&self) -> Result<Vec<SpilledPageData>> {
        let mut pages = Vec::with_capacity(self.spilled_pages.len());
        for path in self.spilled_pages.values() {
            let data = if self.compress {
                let file = File::open(path)?;
                let mut decoder = GzDecoder::new(BufReader::new(file));
                let mut buf = Vec::new();
                decoder.read_to_end(&mut buf)?;
                buf
            } else {
                std::fs::read(path)?
            };
            let page: SpilledPageData = serde_json::from_slice(&data)
                .map_err(|e| PdfError::Other(format!("Spill deserialization failed: {e}")))?;
            pages.push(page);
        }
        Ok(pages)
    }

    /// Return the number of pages that have been spilled to disk.
    #[must_use]
    pub fn spilled_count(&self) -> usize {
        self.spilled_pages.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_backend_auto_small() {
        let b = WriteBackend::auto(10);
        assert_eq!(b, WriteBackend::InMemory);
        assert!(!b.is_constant_memory());
    }

    #[test]
    fn write_backend_auto_large() {
        let b = WriteBackend::auto(200);
        assert!(b.is_constant_memory());
        assert!(matches!(
            b,
            WriteBackend::Spill {
                compress: true,
                threshold_pages: 50,
                ..
            }
        ));
    }

    #[test]
    fn write_backend_constant_memory() {
        let b = WriteBackend::constant_memory();
        assert!(b.is_constant_memory());
        if let WriteBackend::Spill {
            threshold_pages, ..
        } = b
        {
            assert_eq!(threshold_pages, 1);
        } else {
            panic!("expected Spill");
        }
    }

    #[test]
    fn write_backend_default_is_in_memory() {
        assert_eq!(WriteBackend::default(), WriteBackend::InMemory);
    }

    #[test]
    fn spill_writer_below_threshold_is_noop() {
        let mut sw = PageSpillWriter::new(None, false, 5).unwrap();
        let data = SpilledPageData {
            page_number: 1,
            width_pt: 595.0,
            height_pt: 842.0,
            ops: vec![],
        };
        assert!(sw.maybe_spill(&data).unwrap().is_none());
        assert_eq!(sw.spilled_count(), 0);
    }

    #[test]
    fn spill_writer_above_threshold_writes_file() {
        let mut sw = PageSpillWriter::new(None, false, 2).unwrap();
        for i in 1..=3 {
            let data = SpilledPageData {
                page_number: i,
                width_pt: 595.0,
                height_pt: 842.0,
                ops: vec![],
            };
            sw.maybe_spill(&data).unwrap();
        }
        // Pages 3 should be spilled (1 and 2 are at or below threshold).
        assert_eq!(sw.spilled_count(), 1);
    }

    #[test]
    fn spill_writer_roundtrip_with_compression() {
        let mut sw = PageSpillWriter::new(None, true, 0).unwrap();
        for i in 1..=3 {
            let data = SpilledPageData {
                page_number: i,
                width_pt: 595.0 * i as f64,
                height_pt: 842.0,
                ops: vec![],
            };
            sw.maybe_spill(&data).unwrap();
        }
        let collected = sw.collect_all().unwrap();
        assert_eq!(collected.len(), 3);
        assert_eq!(collected[0].page_number, 1);
        assert_eq!(collected[2].page_number, 3);
    }

    #[test]
    fn spill_writer_cleans_up_on_drop() {
        let spill_path;
        {
            let mut sw = PageSpillWriter::new(None, false, 0).unwrap();
            spill_path = sw.spill_dir.clone();
            let data = SpilledPageData {
                page_number: 1,
                width_pt: 595.0,
                height_pt: 842.0,
                ops: vec![],
            };
            sw.maybe_spill(&data).unwrap();
            assert_eq!(sw.spilled_count(), 1);
        }
        // After drop, the temp directory should be cleaned up.
        assert!(
            !spill_path.exists(),
            "spill directory should be cleaned up after drop"
        );
    }
}
