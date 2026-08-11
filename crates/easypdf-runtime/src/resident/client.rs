//! Client for connecting to a resident PDF daemon.
//!
//! Supports both Unix socket and TCP transports. Use [`ResidentClient::connect`]
//! for Unix sockets, [`ResidentClient::connect_tcp`] for TCP, or
//! [`ResidentClient::auto_connect`] for platform-appropriate defaults.

use std::io::{BufRead, BufReader, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use super::error::{ResidentError, Result};
use super::protocol::{
    OpenMode, PageRange, PdfMetadataDto, Request, Response, ResponseData, SessionId,
    MAX_MESSAGE_BYTES,
};

/// Address of the server to connect to.
#[derive(Debug, Clone)]
pub enum TransportAddr {
    /// Unix domain socket path.
    #[cfg(unix)]
    Unix(PathBuf),
    /// TCP socket address (always `127.0.0.1:port`).
    Tcp(SocketAddr),
}

/// Client for communicating with a resident PDF daemon.
///
/// Each method call sends a single request and waits for the response.
/// The client does not hold a persistent connection; instead, each `send()`
/// opens a fresh connection for a single request-response exchange.
///
/// # Examples
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
    /// Connect to a resident daemon at the given Unix socket path.
    ///
    /// # Platform behavior
    ///
    /// On Unix: verifies the socket file exists. On non-Unix platforms:
    /// always returns [`ResidentError::PlatformUnsupported`].
    ///
    /// # Errors
    ///
    /// - [`ResidentError::ServerNotRunning`] if the socket does not exist.
    /// - [`ResidentError::PlatformUnsupported`] on non-Unix platforms.
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

    /// Connect to a resident daemon at the given TCP address.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentError::ServerNotRunning`] if the connection fails.
    pub fn connect_tcp(addr: SocketAddr) -> Result<Self> {
        // Verify the server is reachable
        let _ = std::net::TcpStream::connect_timeout(
            &addr,
            std::time::Duration::from_secs(3),
        )
        .map_err(|_| ResidentError::ServerNotRunning(PathBuf::from(addr.to_string())))?;
        Ok(Self {
            addr: TransportAddr::Tcp(addr),
        })
    }

    /// Connect to a resident daemon by reading the port from the port file.
    ///
    /// This is the TCP equivalent of connecting via a Unix socket path.
    /// The server writes its port to a well-known file; this method reads
    /// it and connects.
    ///
    /// # Errors
    ///
    /// - [`ResidentError::ServerNotRunning`] if the port file does not exist.
    /// - [`ResidentError::Protocol`] if the port file is malformed.
    pub fn connect_tcp_from_port_file() -> Result<Self> {
        let port = super::port::read_port_file()?;
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        Self::connect_tcp(addr)
    }

    /// Connect using the platform-appropriate default.
    ///
    /// - **Unix**: connects to the default socket path.
    /// - **Non-Unix**: reads the port file and connects via TCP.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentError::ServerNotRunning`] if no daemon is found.
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

    /// Check whether a resident daemon is running at the given socket path.
    #[must_use]
    pub fn is_running(socket_path: impl AsRef<Path>) -> bool {
        socket_path.as_ref().exists()
    }

    /// Check whether a resident daemon is running at the given TCP address.
    #[must_use]
    pub fn is_running_tcp(addr: &SocketAddr) -> bool {
        std::net::TcpStream::connect_timeout(addr, std::time::Duration::from_millis(500)).is_ok()
    }

    /// Open a PDF document in the daemon.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentError`] if the request fails.
    pub fn open(&self, path: &str, mode: OpenMode) -> Result<SessionId> {
        let response = self.send(&Request::Open {
            path: path.to_string(),
            mode,
        })?;
        response
            .session_id
            .ok_or_else(|| ResidentError::Protocol("server returned no session id".into()))
    }

    /// Extract text from pages of an open session.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentError`] if the request fails.
    pub fn extract_text(
        &self,
        session: SessionId,
        pages: Option<PageRange>,
    ) -> Result<String> {
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

    /// Extract metadata from an open session.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentError`] if the request fails.
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

    /// Get the page count of an open session.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentError`] if the request fails.
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

    /// Rotate a page in an open session.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentError`] if the request fails.
    pub fn rotate_page(
        &self,
        session: SessionId,
        page: usize,
        rotation: u16,
    ) -> Result<()> {
        let response = self.send(&Request::RotatePage {
            session_id: session,
            page,
            rotation,
        })?;
        check_ok(response)
    }

    /// Save the document in an open session.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentError`] if the request fails.
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

    /// Close a session.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentError`] if the request fails.
    pub fn close(&self, session: SessionId) -> Result<()> {
        let response = self.send(&Request::Close {
            session_id: session,
        })?;
        check_ok(response)
    }

    /// Ping the server to check liveness.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentError`] if the request fails.
    pub fn ping(&self) -> Result<()> {
        let response = self.send(&Request::Ping)?;
        check_ok(response)
    }

    /// Ask the server to shut down gracefully.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentError`] if the request fails.
    pub fn shutdown(&self) -> Result<()> {
        let response = self.send(&Request::Shutdown)?;
        check_ok(response)
    }

    // --- Private helpers ---

    fn send(&self, request: &Request) -> Result<Response> {
        let mut conn: Box<dyn super::transport::Connection> = self.connect_transport()?;

        // Set read/write timeouts
        conn.set_read_timeout(std::time::Duration::from_secs(30))?;

        // Send request as JSON line
        let mut json = serde_json::to_string(&request)
            .map_err(|e| ResidentError::Protocol(format!("serialize failed: {e}")))?;
        json.push('\n');
        conn.write_all(json.as_bytes())?;
        conn.flush()?;

        // Read response line
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
                code: response
                    .error_code
                    .unwrap_or_else(|| "UNKNOWN".to_string()),
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
            code: response
                .error_code
                .unwrap_or_else(|| "UNKNOWN".to_string()),
            message: response
                .error_message
                .unwrap_or_else(|| "unknown error".to_string()),
        })
    }
}
