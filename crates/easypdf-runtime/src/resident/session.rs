//! Document session management.

use std::path::{Path, PathBuf};
use std::time::Instant;

use easypdf_core::Rotation;
use easypdf_reader::{PdfManipulator, PdfReader};
use tracing::{debug, info};

use super::error::{ResidentError, Result};
use super::protocol::{OpenMode, PageRange, PdfMetadataDto, SessionId};

/// A single open document session.
///
/// Wraps either a [`PdfReader`] (read-only) or a [`PdfManipulator`] (read-write),
/// keeping the parsed PDF in memory for the lifetime of the session.
pub struct DocumentSession {
    /// Unique session identifier.
    pub id: SessionId,
    /// File path this session was opened from.
    pub path: PathBuf,
    /// Open mode.
    pub mode: OpenMode,
    /// Reader (always present).
    reader: Option<PdfReader>,
    /// Manipulator (present only in `ReadWrite` mode).
    manipulator: Option<PdfManipulator>,
    /// When the session was created.
    pub opened_at: Instant,
    /// Last time this session was accessed.
    pub last_accessed: Instant,
    /// Whether the document has unsaved modifications.
    pub dirty: bool,
    /// Adaptive autosave EMA of save durations (in seconds). `None` = no sample yet.
    pub save_ema_secs: Option<f64>,
    /// Current autosave interval (adaptive mode).
    pub autosave_interval: Option<std::time::Duration>,
}

impl DocumentSession {
    /// Open a new document session.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentError::Pdf`] if the file cannot be opened or parsed.
    pub fn open(id: SessionId, path: &Path, mode: OpenMode) -> Result<Self> {
        info!(session_id = id, path = %path.display(), ?mode, "opening document session");
        let now = Instant::now();
        let reader = Some(PdfReader::open(path)?);
        let manipulator = match mode {
            OpenMode::ReadWrite => Some(PdfManipulator::open(path)?),
            OpenMode::ReadOnly => None,
        };

        Ok(Self {
            id,
            path: path.to_path_buf(),
            mode,
            reader,
            manipulator,
            opened_at: now,
            last_accessed: now,
            dirty: false,
            save_ema_secs: None,
            autosave_interval: None,
        })
    }

    /// Extract text from the specified page range.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentError::Pdf`] if text extraction fails.
    pub fn extract_text(&mut self, pages: Option<&PageRange>) -> Result<String> {
        debug!(session_id = self.id, ?pages, "extracting text");
        self.touch();
        let reader = self
            .reader
            .as_ref()
            .ok_or_else(|| ResidentError::Protocol("no reader available".into()))?;

        if let Some(range) = pages {
            let count = reader.page_count()?;
            let end = range.end.unwrap_or(count).min(count);
            if range.start >= end {
                return Ok(String::new());
            }
            // Page-range extraction: use per-page extraction when available,
            // fall back to full text extraction for now.
            // TODO: use reader.extract_page_text() per page for precise slicing.
            let _ = (range.start, end);
        }

        // Delegate to full text extraction (covers all pages or filtered above).
        Ok(reader.extract_text()?)
    }

    /// Get the total number of pages.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentError::Pdf`] if the page count cannot be determined.
    pub fn page_count(&mut self) -> Result<usize> {
        debug!(session_id = self.id, "getting page count");
        self.touch();
        let reader = self
            .reader
            .as_ref()
            .ok_or_else(|| ResidentError::Protocol("no reader available".into()))?;
        Ok(reader.page_count()?)
    }

    /// Extract document metadata.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentError::Pdf`] if metadata extraction fails.
    pub fn extract_metadata(&mut self) -> Result<PdfMetadataDto> {
        self.touch();
        let reader = self
            .reader
            .as_ref()
            .ok_or_else(|| ResidentError::Protocol("no reader available".into()))?;
        let meta = reader.extract_metadata()?;
        Ok(PdfMetadataDto {
            title: meta.title,
            author: meta.author,
            subject: meta.subject,
            keywords: meta.keywords,
            creator: meta.creator,
            producer: meta.producer,
        })
    }

    /// Rotate a page.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentError::Pdf`] if the page is invalid.
    /// Returns [`ResidentError::Server`] if the session is read-only.
    pub fn rotate_page(&mut self, page: usize, degrees: u16) -> Result<()> {
        info!(session_id = self.id, page, degrees, "rotating page");
        self.touch();
        if self.mode == OpenMode::ReadOnly {
            return Err(ResidentError::Server {
                code: "READ_ONLY".into(),
                message: "cannot modify a read-only session".into(),
            });
        }
        let rotation = match degrees {
            0 => Rotation::None,
            90 => Rotation::Clockwise90,
            180 => Rotation::Clockwise180,
            270 => Rotation::Clockwise270,
            _ => {
                return Err(ResidentError::Server {
                    code: "INVALID_ROTATION".into(),
                    message: format!("rotation must be 0, 90, 180, or 270, got {degrees}"),
                });
            }
        };
        let manip = self
            .manipulator
            .as_mut()
            .ok_or_else(|| ResidentError::Protocol("no manipulator available".into()))?;
        manip.rotate_page(page, rotation)?;
        self.dirty = true;
        Ok(())
    }

    /// Save the document.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentError::Pdf`] if the save fails.
    /// Returns [`ResidentError::Server`] if the session is read-only.
    pub fn save(&mut self, path: Option<&Path>) -> Result<PathBuf> {
        info!(session_id = self.id, ?path, "saving document");
        self.touch();
        if self.mode == OpenMode::ReadOnly {
            return Err(ResidentError::Server {
                code: "READ_ONLY".into(),
                message: "cannot save a read-only session".into(),
            });
        }
        let manip = self
            .manipulator
            .take()
            .ok_or_else(|| ResidentError::Protocol("no manipulator available".into()))?;

        let save_path = path.unwrap_or(&self.path);
        let save_path_buf = save_path.to_path_buf();
        let start = Instant::now();
        manip.save(save_path)?;
        let elapsed = start.elapsed();
        self.dirty = false;

        // Record save duration for adaptive autosave
        self.record_save_duration(elapsed);

        // Re-open the manipulator so the session stays alive
        self.manipulator = Some(PdfManipulator::open(&self.path)?);
        // Also re-open the reader to reflect saved state
        self.reader = Some(PdfReader::open(&self.path)?);

        Ok(save_path_buf)
    }

    /// Whether the document has unsaved changes.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Update the last-accessed timestamp.
    pub fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }

    /// Record a save duration for adaptive autosave EMA.
    pub fn record_save_duration(&mut self, elapsed: std::time::Duration) {
        const ALPHA: f64 = 0.3;
        const MULTIPLIER: f64 = 4.0;
        const MIN_SECS: f64 = 10.0;
        const MAX_SECS: f64 = 300.0;

        let sample = elapsed.as_secs_f64();
        let ema = match self.save_ema_secs {
            Some(prev) => ALPHA * sample + (1.0 - ALPHA) * prev,
            None => sample,
        };
        self.save_ema_secs = Some(ema);

        let interval_secs = (MULTIPLIER * ema).clamp(MIN_SECS, MAX_SECS);
        self.autosave_interval = Some(std::time::Duration::from_secs_f64(interval_secs));
    }
}
