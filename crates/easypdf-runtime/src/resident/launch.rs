//! Resident daemon launch utilities.
//!
//! 提供便捷函数，用于计算 socket 路径、启动常驻守护进程、以及连接到已运行的守护进程。

use std::path::{Path, PathBuf};

use super::client::ResidentClient;
use super::error::Result;
use super::server::ResidentServer;

/// 计算默认的 Unix socket 路径。
///
/// 使用系统临时目录与固定文件名。若需要按文件隔离，请使用
/// [`socket_path_for_file`]。
///
/// 仅在 Unix 平台上有实际意义。
#[must_use]
pub fn default_socket_path() -> PathBuf {
    std::env::temp_dir().join("easypdf-resident.sock")
}

/// 根据 PDF 文件路径计算独立的 socket 路径。
///
/// 对绝对路径进行哈希以生成唯一的 socket 文件名，
/// 防止不同文档之间的路径冲突。
///
/// 仅在 Unix 平台上有实际意义。
#[must_use]
pub fn socket_path_for_file(pdf_path: &Path) -> PathBuf {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let abs_path = std::fs::canonicalize(pdf_path).unwrap_or_else(|_| pdf_path.to_path_buf());
    let mut hasher = DefaultHasher::new();
    abs_path.to_string_lossy().hash(&mut hasher);
    let hash = hasher.finish();
    std::env::temp_dir().join(format!("easypdf-{hash:x}.sock"))
}

/// 在前台阻塞启动 resident 服务器。
///
/// 这是一个便捷函数，使用默认配置创建并运行服务器。
/// 如需自定义配置，请直接使用 [`ResidentServer`]。
///
/// 在 Unix 上使用给定的 socket 路径（或默认路径）；
/// 在非 Unix 平台上回退到 TCP localhost。
///
/// # Errors
///
/// 如果服务器无法绑定或运行，返回 [`ResidentError`](crate::resident::ResidentError)。
pub fn serve(socket_path: Option<&Path>) -> Result<()> {
    let path = socket_path.map_or_else(default_socket_path, Path::to_path_buf);
    let server = ResidentServer::bind(&path)?;
    eprintln!("easypdf-resident listening on {}", server.transport_addr());
    server.run()
}

/// 尝试连接到已运行的 resident 守护进程。
///
/// 如果在默认 socket 路径（Unix）或端口文件（TCP）上有守护进程正在运行，
/// 返回 `Some(client)`；否则返回 `None`。
#[must_use]
pub fn try_attach() -> Option<ResidentClient> {
    #[cfg(unix)]
    {
        let path = default_socket_path();
        if ResidentClient::is_running(&path) {
            ResidentClient::connect(&path).ok()
        } else {
            None
        }
    }
    #[cfg(not(unix))]
    {
        ResidentClient::auto_connect().ok()
    }
}
