//! MCP server error types.

use thiserror::Error;

/// Result alias for MCP operations.
pub type Result<T> = std::result::Result<T, McpError>;

/// Errors that can occur in the MCP server.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum McpError {
    /// The JSON-RPC request could not be parsed.
    #[error("parse error: {0}")]
    Parse(String),

    /// The request is not a valid JSON-RPC 2.0 request.
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    /// The requested method does not exist.
    #[error("method not found: {0}")]
    MethodNotFound(String),

    /// Tool parameters are missing or invalid.
    #[error("invalid params: {0}")]
    InvalidParams(String),

    /// An internal error occurred during tool execution.
    #[error("internal error: {0}")]
    Internal(String),

    /// The underlying PDF library returned an error.
    #[error("pdf error: {0}")]
    Pdf(#[from] easypdf_core::PdfError),

    /// An I/O error occurred.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl McpError {
    /// Create an `InvalidParams` error with a message.
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::InvalidParams(message.into())
    }

    /// Create an `Internal` error with a message.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    /// Return the JSON-RPC error code for this error variant.
    #[must_use]
    pub const fn error_code(&self) -> i32 {
        match self {
            Self::Parse(_) => super::protocol::ERROR_PARSE,
            Self::InvalidRequest(_) => super::protocol::ERROR_INVALID_REQUEST,
            Self::MethodNotFound(_) => super::protocol::ERROR_METHOD_NOT_FOUND,
            Self::InvalidParams(_) => super::protocol::ERROR_INVALID_PARAMS,
            Self::Internal(_) | Self::Pdf(_) | Self::Io(_) => super::protocol::ERROR_INTERNAL,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes() {
        assert_eq!(McpError::Parse("x".into()).error_code(), -32700);
        assert_eq!(McpError::InvalidRequest("x".into()).error_code(), -32600);
        assert_eq!(McpError::MethodNotFound("x".into()).error_code(), -32601);
        assert_eq!(McpError::InvalidParams("x".into()).error_code(), -32602);
        assert_eq!(McpError::Internal("x".into()).error_code(), -32603);
    }

    #[test]
    fn error_display() {
        let e = McpError::invalid_params("missing path");
        assert!(e.to_string().contains("missing path"));
    }

    #[test]
    fn pdf_error_conversion() {
        let pdf_err = easypdf_core::PdfError::Parse("bad pdf".to_string());
        let mcp_err: McpError = pdf_err.into();
        assert_eq!(mcp_err.error_code(), -32603);
    }
}
