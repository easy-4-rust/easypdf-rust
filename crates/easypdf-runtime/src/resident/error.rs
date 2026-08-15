//! Resident 守护进程的错误类型。

use std::io;
use std::path::PathBuf;

/// Resident 守护进程操作的错误类型。
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ResidentError {
    /// I/O 错误（socket、文件访问）。
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// IPC 协议错误（消息格式错误、序列化失败）。
    #[error("protocol error: {0}")]
    Protocol(String),

    /// 服务器返回了错误响应。
    #[error("server error [{code}]: {message}")]
    Server {
        /// 机器可读的错误码。
        code: String,
        /// 人类可读的错误消息。
        message: String,
    },

    /// 会话未找到（过期或无效的会话 ID）。
    #[error("session {0} not found")]
    SessionNotFound(u64),

    /// 已达到最大会话数上限。
    #[error("maximum sessions ({max}) reached")]
    MaxSessionsReached {
        /// 配置的最大值。
        max: usize,
    },

    /// socket 路径已存在（可能有另一个服务器正在运行）。
    #[error("socket already exists: {0}")]
    SocketAlreadyExists(PathBuf),

    /// 指定 socket 路径上没有服务器在运行。
    #[error("server not running at {0}")]
    ServerNotRunning(PathBuf),

    /// 从读取器/写入器/操作器传播的 PDF 处理错误。
    #[error("PDF error: {0}")]
    Pdf(#[from] easypdf_core::error::PdfError),

    /// 请求超时。
    #[error("request timed out")]
    Timeout,

    /// 请求的传输方式在当前平台上不受支持。
    #[error("operation not supported on this platform: {0}")]
    PlatformUnsupported(String),
}

/// Resident 操作的便捷 `Result` 类型别名。
pub type Result<T, E = ResidentError> = std::result::Result<T, E>;
