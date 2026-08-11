//! TCP localhost transport (cross-platform, primary on Windows).
//!
//! Binds exclusively to `127.0.0.1` to prevent remote connections.
//! On Windows this is the default transport; on Unix it can be used
//! for testing or cross-network scenarios.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};

use super::error::Result;
use super::transport::{Connection, Transport};

/// A TCP listener bound to `127.0.0.1`.
///
/// Created by [`TcpTransport::bind_localhost`] or [`TcpTransport::bind_port`].
/// Implements [`Transport`] so it can be passed to
/// `with_transport`.
pub struct TcpTransport {
    listener: TcpListener,
    port: u16,
}

impl TcpTransport {
    /// Bind to `127.0.0.1` on a random available port.
    ///
    /// The assigned port can be retrieved via [`port()`](TcpTransport::port).
    ///
    /// # Errors
    ///
    /// Returns `ResidentError::Io` if binding fails.
    pub fn bind_localhost() -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        Ok(Self { listener, port })
    }

    /// Bind to `127.0.0.1` on a specific port.
    ///
    /// # Errors
    ///
    /// Returns `ResidentError::Io` if binding fails
    /// (e.g. port already in use).
    pub fn bind_port(port: u16) -> Result<Self> {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let listener = TcpListener::bind(addr)?;
        Ok(Self { listener, port })
    }

    /// The port this transport is listening on.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// The full socket address this transport is listening on.
    #[must_use]
    pub fn addr(&self) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], self.port))
    }
}

impl Transport for TcpTransport {
    fn accept(&self) -> Result<Box<dyn Connection>> {
        let (stream, _addr) = self.listener.accept()?;
        // Set default read timeout for accepted connections
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(30)));
        Ok(Box::new(TcpConnection { stream }))
    }

    fn set_nonblocking(&self, nonblocking: bool) -> Result<()> {
        self.listener.set_nonblocking(nonblocking)?;
        Ok(())
    }

    fn local_addr(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    fn close(&self) {
        // TcpListener is closed on drop; nothing extra to clean up.
    }
}

// --- TcpConnection ---

/// A TCP stream, wrapping [`TcpStream`].
pub struct TcpConnection {
    stream: TcpStream,
}

impl TcpConnection {
    /// Connect to a TCP address.
    ///
    /// # Errors
    ///
    /// Returns `ResidentError::Io` if the connection fails.
    pub fn connect(addr: &SocketAddr) -> Result<Self> {
        let stream = TcpStream::connect(addr)?;
        Ok(Self { stream })
    }
}

impl Read for TcpConnection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.stream.read(buf)
    }
}

impl Write for TcpConnection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.stream.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stream.flush()
    }
}

impl Connection for TcpConnection {
    fn try_clone(&self) -> Result<Box<dyn Connection>> {
        let cloned = self.stream.try_clone()?;
        Ok(Box::new(TcpConnection { stream: cloned }))
    }

    fn set_read_timeout(&self, duration: std::time::Duration) -> Result<()> {
        self.stream.set_read_timeout(Some(duration))?;
        Ok(())
    }

    fn peer_addr(&self) -> String {
        self.stream
            .peer_addr()
            .map_or_else(|_| "tcp-peer".to_string(), |a| a.to_string())
    }
}
