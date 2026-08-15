//! TCP localhost 传输（跨平台，Windows 上的主传输方式）。
//!
//! 仅绑定到 `127.0.0.1` 以阻止远程连接。
//! 在 Windows 上这是默认传输；在 Unix 上可用于测试或跨网络场景。

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};

use super::error::Result;
use super::transport::{Connection, Transport};

/// 绑定到 `127.0.0.1` 的 TCP 监听器。
///
/// 由 [`TcpTransport::bind_localhost`] 或 [`TcpTransport::bind_port`] 创建。
/// 实现了 [`Transport`] trait，因此可以传递给 `with_transport`。
pub struct TcpTransport {
    listener: TcpListener,
    port: u16,
}

impl TcpTransport {
    /// 绑定到 `127.0.0.1` 的随机可用端口。
    ///
    /// 分配的端口可通过 [`port()`](TcpTransport::port) 获取。
    ///
    /// # Errors
    ///
    /// 如果绑定失败，返回 `ResidentError::Io`。
    pub fn bind_localhost() -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        Ok(Self { listener, port })
    }

    /// 绑定到 `127.0.0.1` 的指定端口。
    ///
    /// # Errors
    ///
    /// 如果绑定失败（例如端口已被占用），返回 `ResidentError::Io`。
    pub fn bind_port(port: u16) -> Result<Self> {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let listener = TcpListener::bind(addr)?;
        Ok(Self { listener, port })
    }

    /// 此传输正在监听的端口。
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// 此传输正在监听的完整 socket 地址。
    #[must_use]
    pub fn addr(&self) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], self.port))
    }
}

impl Transport for TcpTransport {
    fn accept(&self) -> Result<Box<dyn Connection>> {
        let (stream, _addr) = self.listener.accept()?;
        // 为接受的连接设置默认读取超时
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
        // TcpListener 在 drop 时关闭；无需额外清理。
    }
}

// --- TcpConnection ---

/// TCP 流，包装 [`TcpStream`]。
pub struct TcpConnection {
    stream: TcpStream,
}

impl TcpConnection {
    /// 连接到指定的 TCP 地址。
    ///
    /// # Errors
    ///
    /// 如果连接失败，返回 `ResidentError::Io`。
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
