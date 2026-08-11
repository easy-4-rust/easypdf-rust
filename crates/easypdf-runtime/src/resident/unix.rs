//! Unix domain socket transport (Linux / macOS).
//!
//! Only compiled on `cfg(unix)` platforms.

use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use super::error::Result;
use super::transport::{Connection, Transport};

/// A Unix domain socket listener.
///
/// Created by [`UnixTransport::bind`]. Implements [`Transport`] so it can be
/// passed to `with_transport`.
pub struct UnixTransport {
    listener: UnixListener,
    path: PathBuf,
}

impl UnixTransport {
    /// Bind to the given Unix socket path.
    ///
    /// Removes any stale socket file at `path` before binding. Sets socket
    /// file permissions to the given `mode` (default `0o600`).
    ///
    /// # Errors
    ///
    /// Returns `ResidentError::Io` if binding fails.
    pub fn bind(path: impl AsRef<Path>) -> Result<Self> {
        Self::bind_with_mode(path, 0o600)
    }

    /// Bind with an explicit permission mode.
    ///
    /// # Errors
    ///
    /// Returns `ResidentError::Io` if binding fails.
    pub fn bind_with_mode(path: impl AsRef<Path>, mode: u32) -> Result<Self> {
        use std::os::unix::fs::PermissionsExt;

        let path = path.as_ref().to_path_buf();

        // Remove stale socket file
        if path.exists() {
            std::fs::remove_file(&path)?;
        }

        let listener = UnixListener::bind(&path)?;

        // Set socket permissions
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode))?;

        Ok(Self { listener, path })
    }

    /// The socket file path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Transport for UnixTransport {
    fn accept(&self) -> Result<Box<dyn Connection>> {
        let (stream, _addr) = self.listener.accept()?;
        Ok(Box::new(UnixConnection { stream }))
    }

    fn set_nonblocking(&self, nonblocking: bool) -> Result<()> {
        self.listener.set_nonblocking(nonblocking)?;
        Ok(())
    }

    fn local_addr(&self) -> String {
        self.path.to_string_lossy().into_owned()
    }

    fn close(&self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

impl Drop for UnixTransport {
    fn drop(&mut self) {
        self.close();
    }
}

// --- UnixConnection ---

/// A Unix domain socket stream, wrapping [`UnixStream`].
pub struct UnixConnection {
    stream: UnixStream,
}

impl UnixConnection {
    /// Connect to a Unix socket at the given path.
    ///
    /// # Errors
    ///
    /// Returns `ResidentError::Io` if the connection fails.
    pub fn connect(path: impl AsRef<Path>) -> Result<Self> {
        let stream = UnixStream::connect(path)?;
        Ok(Self { stream })
    }
}

impl Read for UnixConnection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.stream.read(buf)
    }
}

impl Write for UnixConnection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.stream.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stream.flush()
    }
}

impl Connection for UnixConnection {
    fn try_clone(&self) -> Result<Box<dyn Connection>> {
        let cloned = self.stream.try_clone()?;
        Ok(Box::new(UnixConnection { stream: cloned }))
    }

    fn set_read_timeout(&self, duration: std::time::Duration) -> Result<()> {
        self.stream.set_read_timeout(Some(duration))?;
        Ok(())
    }

    fn peer_addr(&self) -> String {
        // UnixStream doesn't have a meaningful peer_addr display;
        // use a placeholder.
        "unix-peer".to_string()
    }
}
