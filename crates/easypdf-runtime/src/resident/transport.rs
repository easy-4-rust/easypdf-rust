//! IPC 通信的传输抽象层。
//!
//! 提供 [`Transport`]（监听端）和 [`Connection`]（流）trait，
//! 抽象 Unix 域 socket 和 TCP，实现跨平台 IPC。
//!
//! # 平台默认值
//!
//! - **Unix（Linux / macOS）**：通过 Unix 域 socket 的 [`UnixTransport`](super::unix::UnixTransport)。
//! - **Windows**：绑定到 `127.0.0.1`（仅本地）的 [`TcpTransport`](super::tcp::TcpTransport)。

use std::io::{Read, Write};

use super::error::Result;

/// IPC 监听端点的抽象。
///
/// 实现者接受传入连接并产生 [`Connection`] trait 对象。
/// 服务器主循环重复调用 [`accept`](Transport::accept) 以服务客户端。
pub trait Transport: Send {
    /// 接受下一个传入连接。
    ///
    /// # Errors
    ///
    /// 如果接受失败，返回 `ResidentError::Io`。
    fn accept(&self) -> Result<Box<dyn Connection>>;

    /// 将传输设置为非阻塞模式。
    ///
    /// 当非阻塞时，如果没有待处理的连接，
    /// [`accept`](Transport::accept) 返回 [`std::io::ErrorKind::WouldBlock`]。
    ///
    /// # Errors
    ///
    /// 如果操作失败，返回 `ResidentError::Io`。
    fn set_nonblocking(&self, nonblocking: bool) -> Result<()>;

    /// 监听地址的人类可读描述（用于日志记录）。
    fn local_addr(&self) -> String;

    /// 关闭传输，释放绑定的资源。
    fn close(&self);
}

/// 单个 IPC 连接的抽象（客户端流）。
///
/// 组合了 [`Read`] + [`Write`]、连接元数据以及在不同线程中
/// 并发读写时复制句柄的能力。
pub trait Connection: Read + Write + Send {
    /// 复制连接句柄。
    ///
    /// 克隆的句柄共享相同的底层 socket。当服务器将一个句柄
    /// 包装在 [`std::io::BufReader`] 中读取并使用另一个写入时需要此操作。
    ///
    /// # Errors
    ///
    /// 如果克隆失败，返回 `ResidentError::Io`。
    fn try_clone(&self) -> Result<Box<dyn Connection>>;

    /// 设置此连接的读取超时。
    ///
    /// # Errors
    ///
    /// 如果操作失败，返回 `ResidentError::Io`。
    fn set_read_timeout(&self, duration: std::time::Duration) -> Result<()>;

    /// 对端地址的人类可读描述（用于日志记录）。
    fn peer_addr(&self) -> String;
}
