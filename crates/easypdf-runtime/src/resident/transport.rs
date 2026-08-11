//! Transport abstraction for IPC communication.
//!
//! Provides [`Transport`] (listener) and [`Connection`] (stream) traits that
//! abstract over Unix domain sockets and TCP, enabling cross-platform IPC.
//!
//! # Platform defaults
//!
//! - **Unix (Linux / macOS)**: [`UnixTransport`](super::unix::UnixTransport) via Unix domain sockets.
//! - **Windows**: [`TcpTransport`](super::tcp::TcpTransport) bound to `127.0.0.1` (localhost only).

use std::io::{Read, Write};

use super::error::Result;

/// Abstraction over a listening IPC endpoint.
///
/// Implementations accept incoming connections and produce [`Connection`]
/// trait objects. The server main loop calls [`accept`](Transport::accept)
/// repeatedly to service clients.
pub trait Transport: Send {
    /// Accept the next incoming connection.
    ///
    /// # Errors
    ///
    /// Returns `ResidentError::Io` if the accept fails.
    fn accept(&self) -> Result<Box<dyn Connection>>;

    /// Set the transport to non-blocking mode.
    ///
    /// When non-blocking, [`accept`](Transport::accept) returns
    /// [`std::io::ErrorKind::WouldBlock`] if no connection is pending.
    ///
    /// # Errors
    ///
    /// Returns `ResidentError::Io` if the operation fails.
    fn set_nonblocking(&self, nonblocking: bool) -> Result<()>;

    /// Human-readable description of the listening address (for logging).
    fn local_addr(&self) -> String;

    /// Shut down the transport, releasing bound resources.
    fn close(&self);
}

/// Abstraction over a single IPC connection (client stream).
///
/// Combines [`Read`] + [`Write`] with connection metadata and the ability
/// to duplicate the handle for concurrent read/write in different threads.
pub trait Connection: Read + Write + Send {
    /// Duplicate the connection handle.
    ///
    /// The cloned handle shares the same underlying socket. This is needed
    /// when the server wraps one handle in a [`std::io::BufReader`] for
    /// reading and uses the other for writing.
    ///
    /// # Errors
    ///
    /// Returns `ResidentError::Io` if cloning fails.
    fn try_clone(&self) -> Result<Box<dyn Connection>>;

    /// Set the read timeout for this connection.
    ///
    /// # Errors
    ///
    /// Returns `ResidentError::Io` if the operation fails.
    fn set_read_timeout(&self, duration: std::time::Duration) -> Result<()>;

    /// Human-readable description of the peer address (for logging).
    fn peer_addr(&self) -> String;
}
