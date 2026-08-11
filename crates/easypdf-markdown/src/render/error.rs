//! Error types for PDF rendering.

use std::path::PathBuf;

/// Errors that can occur during PDF page rendering.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RenderError {
    /// An I/O error occurred (reading PDF or writing output).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The PDF could not be parsed or is malformed.
    #[error("PDF parse error: {0}")]
    Parse(String),

    /// The requested page index is out of bounds.
    #[error("page index {index} out of bounds (total {total})")]
    InvalidPage {
        /// The requested page index (0-based).
        index: usize,
        /// Total number of pages in the document.
        total: usize,
    },

    /// The requested rendering backend is not available.
    #[error("render backend '{name}' is not available: {reason}")]
    BackendUnavailable {
        /// Backend name.
        name: &'static str,
        /// Why it is unavailable.
        reason: String,
    },

    /// The pdfium dynamic library could not be loaded.
    #[cfg(feature = "pdfium")]
    #[error("pdfium library error: {0}")]
    Pdfium(String),

    /// Image encoding failed (PNG/JPEG).
    #[error("image encoding error: {0}")]
    ImageEncode(String),

    /// The requested DPI exceeds the backend's maximum.
    #[error("DPI {requested} exceeds backend maximum {max}")]
    DpiExceeded {
        /// Requested DPI.
        requested: u32,
        /// Backend maximum DPI.
        max: u32,
    },

    /// The output path is not a valid destination.
    #[error("invalid output path: {0}")]
    InvalidOutput(PathBuf),

    /// Catch-all for other rendering errors.
    #[error("{0}")]
    Other(String),
}

/// Convenience `Result` type for rendering operations.
pub type Result<T, E = RenderError> = std::result::Result<T, E>;
