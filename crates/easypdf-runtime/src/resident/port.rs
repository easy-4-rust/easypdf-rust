//! Port file discovery for TCP transport (Windows fallback).
//!
//! On Windows (or when using TCP transport), the server writes its port
//! number to a file so that clients can discover it. The file is placed
//! in the system temp directory.
//!
//! On Unix this module is available but unused when the default Unix
//! socket transport is active.

use std::path::PathBuf;

use super::error::{ResidentError, Result};

/// Filename for the port discovery file.
const PORT_FILE_NAME: &str = "easypdf-resident.port";

/// Return the path to the port file in the system temp directory.
#[must_use]
pub fn port_file_path() -> PathBuf {
    std::env::temp_dir().join(PORT_FILE_NAME)
}

/// Write the server's port number to the port file.
///
/// Called by the server after binding to a TCP port so that clients
/// can discover which port to connect to.
///
/// # Errors
///
/// Returns [`ResidentError::Io`] if the file cannot be written.
pub fn write_port_file(port: u16) -> Result<()> {
    let path = port_file_path();
    std::fs::write(&path, port.to_string())?;
    Ok(())
}

/// Read the server's port number from the port file.
///
/// Called by the client to discover which TCP port the server is
/// listening on.
///
/// # Errors
///
/// - [`ResidentError::ServerNotRunning`] if the port file does not exist.
/// - [`ResidentError::Protocol`] if the file contents are not a valid port number.
pub fn read_port_file() -> Result<u16> {
    let path = port_file_path();
    let content = std::fs::read_to_string(&path).map_err(|_| {
        ResidentError::ServerNotRunning(path.clone())
    })?;
    let port: u16 = content.trim().parse().map_err(|_| {
        ResidentError::Protocol(format!(
            "invalid port number in {}: {:?}",
            path.display(),
            content.trim()
        ))
    })?;
    Ok(port)
}

/// Remove the port file (best-effort cleanup).
pub fn remove_port_file() {
    let _ = std::fs::remove_file(port_file_path());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_file_path_contains_name() {
        let path = port_file_path();
        assert!(path.to_string_lossy().contains(PORT_FILE_NAME));
    }

    #[test]
    fn test_write_read_roundtrip() {
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
        // This test assumes no port file exists at the default path.
        // If one does exist (e.g. from a running server), the test is a no-op.
        let path = port_file_path();
        if !path.exists() {
            let result = read_port_file();
            assert!(result.is_err());
        }
    }
}
