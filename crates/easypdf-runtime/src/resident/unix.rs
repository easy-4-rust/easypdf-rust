//! Unix 域 socket 传输（Linux / macOS）。
//!
//! 仅在 `cfg(unix)` 平台上编译。

use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use super::error::Result;
use super::transport::{Connection, Transport};

/// Unix 域 socket 监听器。
///
/// 由 [`UnixTransport::bind`] 创建。实现了 [`Transport`] trait，
/// 因此可以传递给 `with_transport`。
pub struct UnixTransport {
    listener: UnixListener,
    path: PathBuf,
}

impl UnixTransport {
    /// 绑定到指定的 Unix socket 路径。
    ///
    /// 绑定前移除 `path` 处的过期 socket 文件。将 socket 文件权限
    /// 设置为给定的 `mode`（默认 `0o600`）。
    ///
    /// # Errors
    ///
    /// 如果绑定失败，返回 `ResidentError::Io`。
    pub fn bind(path: impl AsRef<Path>) -> Result<Self> {
        Self::bind_with_mode(path, 0o600)
    }

    /// 使用显式权限模式绑定。
    ///
    /// # Errors
    ///
    /// 如果绑定失败，返回 `ResidentError::Io`。
    pub fn bind_with_mode(path: impl AsRef<Path>, mode: u32) -> Result<Self> {
        use std::os::unix::fs::PermissionsExt;

        let path = path.as_ref().to_path_buf();

        // 移除过期 socket 文件
        if path.exists() {
            std::fs::remove_file(&path)?;
        }

        let listener = UnixListener::bind(&path)?;

        // 设置 socket 权限
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode))?;

        Ok(Self { listener, path })
    }

    /// socket 文件路径。
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

/// Unix 域 socket 流，包装 [`UnixStream`]。
pub struct UnixConnection {
    stream: UnixStream,
}

impl UnixConnection {
    /// 连接到指定路径的 Unix socket。
    ///
    /// # Errors
    ///
    /// 如果连接失败，返回 `ResidentError::Io`。
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
        // UnixStream 没有有意义的 peer_addr 显示；使用占位符。
        "unix-peer".to_string()
    }
}
