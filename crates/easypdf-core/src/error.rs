//! Error types for `easypdf-rust`.
//!
//! Provides the central `PdfError` enum and a convenience `Result` type alias.

use std::io;

/// Central error type for `easypdf-rust`.
///
/// Covers I/O, parsing, encryption, and unsupported-feature errors.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PdfError {
    /// Wraps a standard I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// A PDF could not be parsed or contains malformed data.
    #[error("PDF parse error: {0}")]
    Parse(String),

    /// A page index is out of bounds.
    #[error("Invalid page index: {0}")]
    InvalidPage(usize),

    /// The requested feature is not yet implemented or not supported by the engine.
    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),

    /// The PDF is encrypted and either no password was supplied or the password was wrong.
    #[error("Encryption error: {0}")]
    Encryption(String),

    /// A configured resource limit was exceeded.
    #[error("Resource limit exceeded for {resource}: actual {actual}, limit {limit}")]
    ResourceLimitExceeded {
        /// Resource name.
        resource: &'static str,
        /// Configured limit.
        limit: u64,
        /// Observed value.
        actual: u64,
    },

    /// Catch-all for other errors.
    #[error("{0}")]
    Other(String),
}

impl PdfError {
    /// Return a stable machine-readable error code.
    #[must_use]
    pub const fn code(&self) -> PdfErrorCode {
        match self {
            Self::Io(_) => PdfErrorCode::Io,
            Self::Parse(_) => PdfErrorCode::Parse,
            Self::InvalidPage(_) => PdfErrorCode::InvalidPage,
            Self::UnsupportedFeature(_) => PdfErrorCode::UnsupportedFeature,
            Self::Encryption(_) => PdfErrorCode::Encryption,
            Self::ResourceLimitExceeded { .. } => PdfErrorCode::ResourceLimitExceeded,
            Self::Other(_) => PdfErrorCode::Other,
        }
    }
}

/// Stable machine-readable PDF error category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PdfErrorCode {
    /// File or stream I/O failure.
    Io,
    /// Malformed or unsupported PDF syntax.
    Parse,
    /// Invalid page selection.
    InvalidPage,
    /// Feature is not implemented by the selected backend.
    UnsupportedFeature,
    /// Encryption or password failure.
    Encryption,
    /// Configured resource limit exceeded.
    ResourceLimitExceeded,
    /// Uncategorized failure.
    Other,
}

/// Convenience `Result` type that uses [`PdfError`] as the error variant.
pub type Result<T, E = PdfError> = std::result::Result<T, E>;
