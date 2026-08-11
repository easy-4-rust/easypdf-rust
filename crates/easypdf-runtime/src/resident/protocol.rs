//! IPC protocol for resident daemon communication.
//!
//! Messages are serialized as JSON, one message per line (newline-delimited JSON).
//! Each request is a single JSON line; each response is a single JSON line.

use serde::{Deserialize, Serialize};

/// Unique identifier for a document session.
pub type SessionId = u64;

/// A range specification for page selection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PageRange {
    /// Start page (0-based, inclusive).
    pub start: usize,
    /// End page (0-based, exclusive). `None` means "to the end".
    pub end: Option<usize>,
}

/// Request from client to server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Request {
    /// Open a PDF document and create a new session.
    Open {
        /// Path to the PDF file.
        path: String,
        /// Open mode (read-only or read-write).
        mode: OpenMode,
    },
    /// Extract text from pages.
    ExtractText {
        /// Target session.
        session_id: SessionId,
        /// Page range (0-based). `None` = all pages.
        pages: Option<PageRange>,
    },
    /// Extract document metadata.
    ExtractMetadata {
        /// Target session.
        session_id: SessionId,
    },
    /// Get total page count.
    PageCount {
        /// Target session.
        session_id: SessionId,
    },
    /// Rotate a page.
    RotatePage {
        /// Target session.
        session_id: SessionId,
        /// Page number (1-based, matching `PdfManipulator` convention).
        page: usize,
        /// Rotation: 0, 90, 180, 270 degrees clockwise.
        rotation: u16,
    },
    /// Save the document.
    Save {
        /// Target session.
        session_id: SessionId,
        /// Optional output path. `None` = save to original path.
        path: Option<String>,
    },
    /// Close a session and release resources.
    Close {
        /// Target session.
        session_id: SessionId,
    },
    /// Liveness probe.
    Ping,
    /// Gracefully shut down the server.
    Shutdown,
}

/// Response from server to client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// Whether the request succeeded.
    pub ok: bool,
    /// Session id (present when the response is session-scoped).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    /// Response payload (present on success).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<ResponseData>,
    /// Error code (present on failure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    /// Error message (present on failure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Successful response payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ResponseData {
    /// Text extraction result.
    Text {
        /// The extracted text content.
        content: String,
    },
    /// Document metadata.
    Metadata {
        /// The document metadata.
        #[serde(flatten)]
        metadata: PdfMetadataDto,
    },
    /// Page count.
    PageCount {
        /// Number of pages.
        count: usize,
    },
    /// Document saved successfully.
    Saved {
        /// The path the document was saved to.
        path: String,
    },
    /// Pong response.
    Pong,
    /// Empty acknowledgement.
    None,
}

/// Metadata DTO for serialization over IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfMetadataDto {
    /// Document title.
    pub title: Option<String>,
    /// Document author.
    pub author: Option<String>,
    /// Document subject.
    pub subject: Option<String>,
    /// Document keywords.
    pub keywords: Option<String>,
    /// Creator application.
    pub creator: Option<String>,
    /// Producer application.
    pub producer: Option<String>,
}

/// Open mode for a document session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpenMode {
    /// Read-only access (no modifications allowed).
    ReadOnly,
    /// Read-write access (modifications and save allowed).
    ReadWrite,
}

// --- Response constructors ---

impl Response {
    /// Create a success response with no payload.
    #[must_use]
    pub fn ok() -> Self {
        Self {
            ok: true,
            session_id: None,
            data: None,
            error_code: None,
            error_message: None,
        }
    }

    /// Create a success response with a session id.
    #[must_use]
    pub fn ok_session(session_id: SessionId) -> Self {
        Self {
            ok: true,
            session_id: Some(session_id),
            data: None,
            error_code: None,
            error_message: None,
        }
    }

    /// Create a success response with data.
    #[must_use]
    pub fn ok_data(session_id: Option<SessionId>, data: ResponseData) -> Self {
        Self {
            ok: true,
            session_id,
            data: Some(data),
            error_code: None,
            error_message: None,
        }
    }

    /// Create an error response.
    #[must_use]
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            ok: false,
            session_id: None,
            data: None,
            error_code: Some(code.into()),
            error_message: Some(message.into()),
        }
    }
}

/// Maximum message size (1 MB) to prevent memory exhaustion from malformed clients.
pub const MAX_MESSAGE_BYTES: usize = 1024 * 1024;
