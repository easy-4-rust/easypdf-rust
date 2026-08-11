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

    /// A security guard rejected the input (decompression bomb, SSRF, etc.).
    #[error("security violation: {0}")]
    SecurityViolation(String),

    /// A digital signature operation failed.
    #[error("signature error: {0}")]
    Signature(String),

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
            Self::SecurityViolation(_) => PdfErrorCode::SecurityViolation,
            Self::Signature(_) => PdfErrorCode::Signature,
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
    /// A security guard rejected the input.
    SecurityViolation,
    /// Digital signature failure.
    Signature,
    /// Uncategorized failure.
    Other,
}

/// Convenience `Result` type that uses [`PdfError`] as the error variant.
pub type Result<T, E = PdfError> = std::result::Result<T, E>;

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn io_error_display() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file missing");
        let err = PdfError::Io(io_err);
        assert!(format!("{}", err).contains("I/O error"));
        assert!(format!("{}", err).contains("file missing"));
    }

    #[test]
    fn io_error_from_conversion() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
        let err: PdfError = io_err.into();
        assert!(matches!(err, PdfError::Io(_)));
    }

    #[test]
    fn parse_error_display() {
        let err = PdfError::Parse("bad header".to_string());
        assert_eq!(format!("{}", err), "PDF parse error: bad header");
    }

    #[test]
    fn invalid_page_display() {
        let err = PdfError::InvalidPage(42);
        assert_eq!(format!("{}", err), "Invalid page index: 42");
    }

    #[test]
    fn unsupported_feature_display() {
        let err = PdfError::UnsupportedFeature("encryption".to_string());
        assert_eq!(format!("{}", err), "Unsupported feature: encryption");
    }

    #[test]
    fn encryption_error_display() {
        let err = PdfError::Encryption("wrong password".to_string());
        assert_eq!(format!("{}", err), "Encryption error: wrong password");
    }

    #[test]
    fn resource_limit_exceeded_display() {
        let err = PdfError::ResourceLimitExceeded {
            resource: "input_bytes",
            limit: 1024,
            actual: 2048,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("input_bytes"));
        assert!(msg.contains("1024"));
        assert!(msg.contains("2048"));
    }

    #[test]
    fn security_violation_display() {
        let err = PdfError::SecurityViolation("SSRF blocked".to_string());
        assert_eq!(format!("{}", err), "security violation: SSRF blocked");
    }

    #[test]
    fn signature_error_display() {
        let err = PdfError::Signature("invalid cert".to_string());
        assert_eq!(format!("{}", err), "signature error: invalid cert");
    }

    #[test]
    fn other_error_display() {
        let err = PdfError::Other("something".to_string());
        assert_eq!(format!("{}", err), "something");
    }

    // --- error code tests ---

    #[test]
    fn code_io() {
        let err = PdfError::Io(io::Error::other("x"));
        assert_eq!(err.code(), PdfErrorCode::Io);
    }

    #[test]
    fn code_parse() {
        let err = PdfError::Parse("x".to_string());
        assert_eq!(err.code(), PdfErrorCode::Parse);
    }

    #[test]
    fn code_invalid_page() {
        let err = PdfError::InvalidPage(0);
        assert_eq!(err.code(), PdfErrorCode::InvalidPage);
    }

    #[test]
    fn code_unsupported_feature() {
        let err = PdfError::UnsupportedFeature("x".to_string());
        assert_eq!(err.code(), PdfErrorCode::UnsupportedFeature);
    }

    #[test]
    fn code_encryption() {
        let err = PdfError::Encryption("x".to_string());
        assert_eq!(err.code(), PdfErrorCode::Encryption);
    }

    #[test]
    fn code_resource_limit_exceeded() {
        let err = PdfError::ResourceLimitExceeded {
            resource: "pages",
            limit: 100,
            actual: 200,
        };
        assert_eq!(err.code(), PdfErrorCode::ResourceLimitExceeded);
    }

    #[test]
    fn code_security_violation() {
        let err = PdfError::SecurityViolation("x".to_string());
        assert_eq!(err.code(), PdfErrorCode::SecurityViolation);
    }

    #[test]
    fn code_signature() {
        let err = PdfError::Signature("x".to_string());
        assert_eq!(err.code(), PdfErrorCode::Signature);
    }

    #[test]
    fn code_other() {
        let err = PdfError::Other("x".to_string());
        assert_eq!(err.code(), PdfErrorCode::Other);
    }

    // --- PdfErrorCode tests ---

    #[test]
    fn error_code_clone() {
        let code = PdfErrorCode::Io;
        let cloned = code;
        assert_eq!(code, cloned);
    }

    #[test]
    fn error_code_debug() {
        assert_eq!(format!("{:?}", PdfErrorCode::Parse), "Parse");
        assert_eq!(format!("{:?}", PdfErrorCode::Io), "Io");
    }

    #[test]
    fn error_code_eq() {
        assert_eq!(PdfErrorCode::Io, PdfErrorCode::Io);
        assert_ne!(PdfErrorCode::Io, PdfErrorCode::Parse);
    }

    #[test]
    fn error_code_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(PdfErrorCode::Io);
        set.insert(PdfErrorCode::Io);
        set.insert(PdfErrorCode::Parse);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn debug_format_all_variants() {
        let errors = vec![
            PdfError::Parse("p".to_string()),
            PdfError::InvalidPage(0),
            PdfError::UnsupportedFeature("u".to_string()),
            PdfError::Encryption("e".to_string()),
            PdfError::SecurityViolation("s".to_string()),
            PdfError::Signature("s".to_string()),
            PdfError::Other("o".to_string()),
        ];
        for err in errors {
            let _ = format!("{:?}", err);
        }
    }
}
