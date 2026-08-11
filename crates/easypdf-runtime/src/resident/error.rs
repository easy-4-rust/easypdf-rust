//! Error types for the resident daemon.

use std::io;
use std::path::PathBuf;

/// Error type for resident daemon operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ResidentError {
    /// I/O error (socket, file access).
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// IPC protocol error (malformed message, serialization failure).
    #[error("protocol error: {0}")]
    Protocol(String),

    /// The server returned an error response.
    #[error("server error [{code}]: {message}")]
    Server {
        /// Machine-readable error code.
        code: String,
        /// Human-readable error message.
        message: String,
    },

    /// The session was not found (stale or invalid session id).
    #[error("session {0} not found")]
    SessionNotFound(u64),

    /// The maximum number of sessions has been reached.
    #[error("maximum sessions ({max}) reached")]
    MaxSessionsReached {
        /// Configured maximum.
        max: usize,
    },

    /// The socket path already exists (another server may be running).
    #[error("socket already exists: {0}")]
    SocketAlreadyExists(PathBuf),

    /// The server is not running at the given socket path.
    #[error("server not running at {0}")]
    ServerNotRunning(PathBuf),

    /// PDF processing error propagated from the reader/writer/manipulator.
    #[error("PDF error: {0}")]
    Pdf(#[from] easypdf_core::error::PdfError),

    /// The request timed out.
    #[error("request timed out")]
    Timeout,

    /// The requested transport is not supported on this platform.
    #[error("operation not supported on this platform: {0}")]
    PlatformUnsupported(String),
}

/// Convenience `Result` type for resident operations.
pub type Result<T, E = ResidentError> = std::result::Result<T, E>;
