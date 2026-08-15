//! Resident 守护进程客户端。
//!
//! 支持 Unix socket 和 TCP 两种传输方式。使用 [`ResidentClient::connect`]
//! 连接 Unix socket，[`ResidentClient::connect_tcp`] 连接 TCP，
//! 或 [`ResidentClient::auto_connect`] 自动选择平台默认方式。

use std::io::{BufRead, BufReader, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use super::error::{ResidentError, Result};
use super::protocol::{
    MAX_MESSAGE_BYTES, OpenMode, PageRange, PdfMetadataDto, Request, Response, ResponseData,
    SessionId,
};

/// 要连接的服务器地址。
#[derive(Debug, Clone)]
pub enum TransportAddr {
    /// Unix 域 socket 路径。
    #[cfg(unix)]
    Unix(PathBuf),
    /// TCP socket 地址（始终为 `127.0.0.1:port`）。
    Tcp(SocketAddr),
}

/// 用于与 resident PDF 守护进程通信的客户端。
///
/// 每次方法调用发送一个请求并等待响应。
/// 客户端不保持持久连接；每次 `send()` 为单次请求-响应交换打开一个新连接。
///
/// # 示例
///
/// ```no_run
/// use easypdf_runtime::resident::{ResidentClient, OpenMode};
///
/// // Unix socket:
/// let client = ResidentClient::connect("/tmp/easypdf.sock")?;
/// let session = client.open("document.pdf", OpenMode::ReadOnly)?;
/// let text = client.extract_text(session, None)?;
/// client.close(session)?;
/// # Ok::<(), easypdf_runtime::resident::ResidentError>(())
/// ```
#[derive(Debug)]
pub struct ResidentClient {
    addr: TransportAddr,
}

impl ResidentClient {
    /// 连接到指定 Unix socket 路径的 resident 守护进程。
    ///
    /// # 平台行为
    ///
    /// 在 Unix 上：验证 socket 文件是否存在。在非 Unix 平台上：
    /// 始终返回 [`ResidentError::PlatformUnsupported`]。
    ///
    /// # Errors
    ///
    /// - 如果 socket 不存在，返回 [`ResidentError::ServerNotRunning`]。
    /// - 在非 Unix 平台上返回 [`ResidentError::PlatformUnsupported`]。
    pub fn connect(socket_path: impl AsRef<Path>) -> Result<Self> {
        #[cfg(unix)]
        {
            let socket_path = socket_path.as_ref().to_path_buf();
            if !socket_path.exists() {
                return Err(ResidentError::ServerNotRunning(socket_path));
            }
            Ok(Self {
                addr: TransportAddr::Unix(socket_path),
            })
        }
        #[cfg(not(unix))]
        {
            let _ = socket_path;
            Err(ResidentError::PlatformUnsupported(
                "Unix sockets are not available on this platform; use connect_tcp()".into(),
            ))
        }
    }

    /// 连接到指定 TCP 地址的 resident 守护进程。
    ///
    /// # Errors
    ///
    /// 如果连接失败，返回 [`ResidentError::ServerNotRunning`]。
    pub fn connect_tcp(addr: SocketAddr) -> Result<Self> {
        let _ = std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(3))
            .map_err(|_| ResidentError::ServerNotRunning(PathBuf::from(addr.to_string())))?;
        Ok(Self {
            addr: TransportAddr::Tcp(addr),
        })
    }

    /// 通过端口文件连接到 resident 守护进程。
    ///
    /// 这是 TCP 方式下等价于通过 Unix socket 路径连接的方法。
    /// 服务器将其端口号写入一个已知文件；此方法读取该文件并连接。
    ///
    /// # Errors
    ///
    /// - 如果端口文件不存在，返回 [`ResidentError::ServerNotRunning`]。
    /// - 如果端口文件格式错误，返回 [`ResidentError::Protocol`]。
    pub fn connect_tcp_from_port_file() -> Result<Self> {
        let port = super::port::read_port_file()?;
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        Self::connect_tcp(addr)
    }

    /// 使用平台默认方式连接。
    ///
    /// - **Unix**：连接到默认 socket 路径。
    /// - **非 Unix**：读取端口文件并通过 TCP 连接。
    ///
    /// # Errors
    ///
    /// 如果未找到守护进程，返回 [`ResidentError::ServerNotRunning`]。
    pub fn auto_connect() -> Result<Self> {
        #[cfg(unix)]
        {
            Self::connect(super::default_socket_path())
        }
        #[cfg(not(unix))]
        {
            Self::connect_tcp_from_port_file()
        }
    }

    /// 检查指定 socket 路径是否有 resident 守护进程正在运行。
    #[must_use]
    pub fn is_running(socket_path: impl AsRef<Path>) -> bool {
        socket_path.as_ref().exists()
    }

    /// 检查指定 TCP 地址是否有 resident 守护进程正在运行。
    #[must_use]
    pub fn is_running_tcp(addr: &SocketAddr) -> bool {
        std::net::TcpStream::connect_timeout(addr, std::time::Duration::from_millis(500)).is_ok()
    }

    /// 在守护进程中打开一个 PDF 文档。
    ///
    /// # Errors
    ///
    /// 如果请求失败，返回 [`ResidentError`]。
    pub fn open(&self, path: &str, mode: OpenMode) -> Result<SessionId> {
        let response = self.send(&Request::Open {
            path: path.to_string(),
            mode,
        })?;
        response
            .session_id
            .ok_or_else(|| ResidentError::Protocol("server returned no session id".into()))
    }

    /// 从打开的会话中提取指定页面的文本。
    ///
    /// # 参数
    ///
    /// * `session` - 会话标识符。
    /// * `pages` - 页面范围（0 起始），`None` 表示所有页面。
    ///
    /// # Errors
    ///
    /// 如果请求失败，返回 [`ResidentError`]。
    pub fn extract_text(&self, session: SessionId, pages: Option<PageRange>) -> Result<String> {
        let response = self.send(&Request::ExtractText {
            session_id: session,
            pages,
        })?;
        match response.data {
            Some(ResponseData::Text { content }) => Ok(content),
            _ => Err(ResidentError::Protocol(
                "unexpected response data for ExtractText".into(),
            )),
        }
    }

    /// 从打开的会话中提取文档元数据。
    ///
    /// # Errors
    ///
    /// 如果请求失败，返回 [`ResidentError`]。
    pub fn extract_metadata(&self, session: SessionId) -> Result<PdfMetadataDto> {
        let response = self.send(&Request::ExtractMetadata {
            session_id: session,
        })?;
        match response.data {
            Some(ResponseData::Metadata { metadata }) => Ok(metadata),
            _ => Err(ResidentError::Protocol(
                "unexpected response data for ExtractMetadata".into(),
            )),
        }
    }

    /// 获取打开会话的总页数。
    ///
    /// # Errors
    ///
    /// 如果请求失败，返回 [`ResidentError`]。
    pub fn page_count(&self, session: SessionId) -> Result<usize> {
        let response = self.send(&Request::PageCount {
            session_id: session,
        })?;
        match response.data {
            Some(ResponseData::PageCount { count }) => Ok(count),
            _ => Err(ResidentError::Protocol(
                "unexpected response data for PageCount".into(),
            )),
        }
    }

    /// 旋转打开会话中的某个页面。
    ///
    /// # Errors
    ///
    /// 如果请求失败，返回 [`ResidentError`]。
    pub fn rotate_page(&self, session: SessionId, page: usize, rotation: u16) -> Result<()> {
        let response = self.send(&Request::RotatePage {
            session_id: session,
            page,
            rotation,
        })?;
        check_ok(response)
    }

    /// 保存打开会话中的文档。
    ///
    /// # 参数
    ///
    /// * `session` - 会话标识符。
    /// * `path` - 可选的输出路径，`None` 表示保存到原始路径。
    ///
    /// # Errors
    ///
    /// 如果请求失败，返回 [`ResidentError`]。
    pub fn save(&self, session: SessionId, path: Option<&str>) -> Result<String> {
        let response = self.send(&Request::Save {
            session_id: session,
            path: path.map(ToOwned::to_owned),
        })?;
        match response.data {
            Some(ResponseData::Saved { path }) => Ok(path),
            _ => Err(ResidentError::Protocol(
                "unexpected response data for Save".into(),
            )),
        }
    }

    /// 关闭一个会话。
    ///
    /// # Errors
    ///
    /// 如果请求失败，返回 [`ResidentError`]。
    pub fn close(&self, session: SessionId) -> Result<()> {
        let response = self.send(&Request::Close {
            session_id: session,
        })?;
        check_ok(response)
    }

    /// 向服务器发送 ping 以检查存活状态。
    ///
    /// # Errors
    ///
    /// 如果请求失败，返回 [`ResidentError`]。
    pub fn ping(&self) -> Result<()> {
        let response = self.send(&Request::Ping)?;
        check_ok(response)
    }

    /// 请求服务器优雅关闭。
    ///
    /// # Errors
    ///
    /// 如果请求失败，返回 [`ResidentError`]。
    pub fn shutdown(&self) -> Result<()> {
        let response = self.send(&Request::Shutdown)?;
        check_ok(response)
    }

    // --- 私有辅助方法 ---

    fn send(&self, request: &Request) -> Result<Response> {
        let mut conn: Box<dyn super::transport::Connection> = self.connect_transport()?;

        // 设置读写超时
        conn.set_read_timeout(std::time::Duration::from_secs(30))?;

        // 将请求序列化为 JSON 行发送
        let mut json = serde_json::to_string(&request)
            .map_err(|e| ResidentError::Protocol(format!("serialize failed: {e}")))?;
        json.push('\n');
        conn.write_all(json.as_bytes())?;
        conn.flush()?;

        // 读取响应行
        let mut buf_reader = BufReader::new(conn);
        let mut line = String::new();
        buf_reader.read_line(&mut line)?;

        if line.len() > MAX_MESSAGE_BYTES {
            return Err(ResidentError::Protocol("response too large".into()));
        }

        let response: Response = serde_json::from_str(line.trim())
            .map_err(|e| ResidentError::Protocol(format!("deserialize failed: {e}")))?;

        if !response.ok {
            return Err(ResidentError::Server {
                code: response.error_code.unwrap_or_else(|| "UNKNOWN".to_string()),
                message: response
                    .error_message
                    .unwrap_or_else(|| "unknown error".to_string()),
            });
        }

        Ok(response)
    }

    fn connect_transport(&self) -> Result<Box<dyn super::transport::Connection>> {
        match &self.addr {
            #[cfg(unix)]
            TransportAddr::Unix(path) => {
                let conn = super::unix::UnixConnection::connect(path)?;
                Ok(Box::new(conn))
            }
            TransportAddr::Tcp(addr) => {
                let conn = super::tcp::TcpConnection::connect(addr)?;
                Ok(Box::new(conn))
            }
        }
    }
}

fn check_ok(response: Response) -> Result<()> {
    if response.ok {
        Ok(())
    } else {
        Err(ResidentError::Server {
            code: response.error_code.unwrap_or_else(|| "UNKNOWN".to_string()),
            message: response
                .error_message
                .unwrap_or_else(|| "unknown error".to_string()),
        })
    }
}
