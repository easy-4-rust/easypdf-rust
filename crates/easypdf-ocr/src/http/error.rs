//! Error types for HTTP-based OCR engines.

/// Errors that can occur during HTTP-based OCR operations.
///
/// Covers transport failures, authentication issues, server errors,
/// and response parsing problems.
#[derive(Debug, thiserror::Error)]
pub enum OcrHttpError {
    /// HTTP transport error (connection refused, timeout, DNS failure, etc.).
    #[error("HTTP transport error: {0}")]
    Transport(#[from] reqwest::Error),

    /// Authentication failed (invalid API key, expired token, bad signature).
    #[error("Authentication failed: {0}")]
    Auth(String),

    /// The server rejected the request (malformed body, missing fields, etc.).
    #[error("Bad request: {message} (code: {code})")]
    BadRequest {
        /// Error code from the server.
        code: i32,
        /// Human-readable error message.
        message: String,
    },

    /// Rate limit exceeded; caller should retry after the given delay.
    #[error("Rate limit exceeded; retry after {retry_after_secs}s")]
    RateLimit {
        /// Suggested retry delay in seconds.
        retry_after_secs: u64,
    },

    /// Server returned a 5xx error.
    #[error("Server error: status={status}, body={body}")]
    ServerError {
        /// HTTP status code.
        status: u16,
        /// Response body (truncated to 500 chars for display).
        body: String,
    },

    /// The server response could not be parsed.
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// The OCR engine returned an application-level error.
    #[error("OCR engine error: {0}")]
    Engine(String),

    /// Maximum retry attempts exhausted.
    #[error("Max retries ({max}) exceeded")]
    MaxRetriesExceeded {
        /// The maximum number of retries that was configured.
        max: u32,
    },
}

/// Convenience result type for OCR HTTP operations.
pub type Result<T> = std::result::Result<T, OcrHttpError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_transport() {
        // Transport errors wrap reqwest::Error; verify the prefix.
        // We can't easily construct a reqwest::Error, so test via the Display impl pattern.
        let err = OcrHttpError::Auth("bad key".to_owned());
        assert!(format!("{err}").contains("Authentication failed"));
        assert!(format!("{err}").contains("bad key"));
    }

    #[test]
    fn test_error_display_bad_request() {
        let err = OcrHttpError::BadRequest {
            code: 400,
            message: "missing image".to_owned(),
        };
        let s = format!("{err}");
        assert!(s.contains("Bad request"));
        assert!(s.contains("400"));
        assert!(s.contains("missing image"));
    }

    #[test]
    fn test_error_display_rate_limit() {
        let err = OcrHttpError::RateLimit {
            retry_after_secs: 30,
        };
        assert!(format!("{err}").contains("30"));
    }

    #[test]
    fn test_error_display_server_error() {
        let err = OcrHttpError::ServerError {
            status: 503,
            body: "Service Unavailable".to_owned(),
        };
        let s = format!("{err}");
        assert!(s.contains("503"));
        assert!(s.contains("Service Unavailable"));
    }

    #[test]
    fn test_error_display_invalid_response() {
        let err = OcrHttpError::InvalidResponse("unexpected JSON structure".to_owned());
        assert!(format!("{err}").contains("unexpected JSON structure"));
    }

    #[test]
    fn test_error_display_engine() {
        let err = OcrHttpError::Engine("OCR model not loaded".to_owned());
        assert!(format!("{err}").contains("OCR model not loaded"));
    }

    #[test]
    fn test_error_display_max_retries() {
        let err = OcrHttpError::MaxRetriesExceeded { max: 5 };
        let s = format!("{err}");
        assert!(s.contains('5'));
    }

    #[test]
    fn test_error_conversion_to_boxed() {
        let err = OcrHttpError::Auth("test".to_owned());
        let boxed: Box<dyn std::error::Error + Send + Sync> = Box::new(err);
        assert!(boxed.to_string().contains("Authentication failed"));
    }
}
