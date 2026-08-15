//! MCP 服务器错误类型。

use thiserror::Error;

/// MCP 操作的 Result 类型别名。
pub type Result<T> = std::result::Result<T, McpError>;

/// MCP 服务器中可能发生的错误。
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum McpError {
    /// 无法解析 JSON-RPC 请求。
    #[error("parse error: {0}")]
    Parse(String),

    /// 请求不是有效的 JSON-RPC 2.0 请求。
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    /// 请求的方法不存在。
    #[error("method not found: {0}")]
    MethodNotFound(String),

    /// 工具参数缺失或无效。
    #[error("invalid params: {0}")]
    InvalidParams(String),

    /// 工具执行期间发生内部错误。
    #[error("internal error: {0}")]
    Internal(String),

    /// 底层 PDF 库返回的错误。
    #[error("pdf error: {0}")]
    Pdf(#[from] easypdf_core::PdfError),

    /// I/O 错误。
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl McpError {
    /// 创建一个 `InvalidParams` 错误。
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::InvalidParams(message.into())
    }

    /// 创建一个 `Internal` 错误。
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    /// 返回此错误变体对应的 JSON-RPC 错误码。
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
