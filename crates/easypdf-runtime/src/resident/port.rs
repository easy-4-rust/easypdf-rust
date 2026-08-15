//! TCP 传输的端口文件发现机制（Windows 回退方案）。
//!
//! 在 Windows（或使用 TCP 传输时），服务器将其端口号写入文件，
//! 以便客户端发现。文件放置在系统临时目录中。
//!
//! 在 Unix 上，此模块可用但当默认的 Unix socket 传输激活时不使用。

use std::path::PathBuf;

use super::error::{ResidentError, Result};

/// 端口发现文件的文件名。
const PORT_FILE_NAME: &str = "easypdf-resident.port";

/// 返回系统临时目录中端口文件的路径。
#[must_use]
pub fn port_file_path() -> PathBuf {
    std::env::temp_dir().join(PORT_FILE_NAME)
}

/// 将服务器端口号写入端口文件。
///
/// 由服务器在绑定到 TCP 端口后调用，以便客户端可以发现
/// 要连接的端口。
///
/// # Errors
///
/// 如果文件无法写入，返回 [`ResidentError::Io`]。
pub fn write_port_file(port: u16) -> Result<()> {
    let path = port_file_path();
    std::fs::write(&path, port.to_string())?;
    Ok(())
}

/// 从端口文件读取服务器端口号。
///
/// 由客户端调用以发现服务器正在监听的 TCP 端口。
///
/// # Errors
///
/// - 如果端口文件不存在，返回 [`ResidentError::ServerNotRunning`]。
/// - 如果文件内容不是有效的端口号，返回 [`ResidentError::Protocol`]。
pub fn read_port_file() -> Result<u16> {
    let path = port_file_path();
    let content = std::fs::read_to_string(&path)
        .map_err(|_| ResidentError::ServerNotRunning(path.clone()))?;
    let port: u16 = content.trim().parse().map_err(|_| {
        ResidentError::Protocol(format!(
            "invalid port number in {}: {:?}",
            path.display(),
            content.trim()
        ))
    })?;
    Ok(port)
}

/// 移除端口文件（尽力清理）。
pub fn remove_port_file() {
    let _ = std::fs::remove_file(port_file_path());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 端口文件测试共享同一个全局路径，必须串行执行避免 TOCTOU 竞争。
    static PORT_FILE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_port_file_path_contains_name() {
        let path = port_file_path();
        assert!(path.to_string_lossy().contains(PORT_FILE_NAME));
    }

    #[test]
    fn test_write_read_roundtrip() {
        let _guard = PORT_FILE_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // Use a unique suffix to avoid collisions
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let original = port_file_path();
        let backup = original.with_extension(format!("port.bak.{ts}"));

        // Backup existing file if present
        let had_backup = original.exists();
        if had_backup {
            let _ = std::fs::rename(&original, &backup);
        }

        // Write and read back
        write_port_file(12345).unwrap();
        let port = read_port_file().unwrap();
        assert_eq!(port, 12345);

        // Cleanup
        remove_port_file();
        assert!(!original.exists());

        // Restore backup
        if had_backup {
            let _ = std::fs::rename(&backup, &original);
        }
    }

    #[test]
    fn test_read_port_file_not_found() {
        let _guard = PORT_FILE_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // This test assumes no port file exists at the default path.
        // If one does exist (e.g. from a running server), the test is a no-op.
        let path = port_file_path();
        if !path.exists() {
            let result = read_port_file();
            assert!(result.is_err());
        }
    }
}
