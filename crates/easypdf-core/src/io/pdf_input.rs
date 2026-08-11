//! PDF 输入来源。

use std::path::{Path, PathBuf};

use crate::{PdfError, Result};

use crate::ResourceLimits;

/// 可从文件路径或内存字节读取的 PDF 输入。
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum PdfInput {
    /// 文件系统路径。
    Path(PathBuf),
    /// 已在内存中的 PDF 字节。
    Bytes(Vec<u8>),
}

impl PdfInput {
    /// 创建路径输入。
    #[must_use]
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self::Path(path.into())
    }

    /// 创建内存字节输入。
    #[must_use]
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Self {
        Self::Bytes(bytes.into())
    }

    /// 在资源限制内读取全部输入字节。
    ///
    /// # Errors
    ///
    /// 输入不可读或超过字节上限时返回错误。
    pub fn read(&self, limits: ResourceLimits) -> Result<Vec<u8>> {
        match self {
            Self::Path(path) => {
                let metadata = std::fs::metadata(path)?;
                if metadata.len() > limits.max_input_bytes() {
                    return Err(PdfError::ResourceLimitExceeded {
                        resource: "input_bytes",
                        limit: limits.max_input_bytes(),
                        actual: metadata.len(),
                    });
                }
                let bytes = std::fs::read(path)?;
                let actual = u64::try_from(bytes.len()).map_err(|_| {
                    PdfError::Other("PDF input length cannot be represented as u64".to_string())
                })?;
                if actual > limits.max_input_bytes() {
                    return Err(PdfError::ResourceLimitExceeded {
                        resource: "input_bytes",
                        limit: limits.max_input_bytes(),
                        actual,
                    });
                }
                Ok(bytes)
            }
            Self::Bytes(bytes) => {
                let length = u64::try_from(bytes.len()).map_err(|_| {
                    PdfError::Other("PDF input length cannot be represented as u64".to_string())
                })?;
                if length > limits.max_input_bytes() {
                    return Err(PdfError::ResourceLimitExceeded {
                        resource: "input_bytes",
                        limit: limits.max_input_bytes(),
                        actual: length,
                    });
                }
                Ok(bytes.clone())
            }
        }
    }

    /// 当输入来自文件系统时返回路径。
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Path(path) => Some(path.as_path()),
            Self::Bytes(_) => None,
        }
    }
}

impl From<PathBuf> for PdfInput {
    fn from(value: PathBuf) -> Self {
        Self::Path(value)
    }
}

impl From<&Path> for PdfInput {
    fn from(value: &Path) -> Self {
        Self::Path(value.to_path_buf())
    }
}

impl From<Vec<u8>> for PdfInput {
    fn from(value: Vec<u8>) -> Self {
        Self::Bytes(value)
    }
}

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn from_path_creates_path_variant() {
        let input = PdfInput::from_path("/tmp/test.pdf");
        assert!(matches!(input, PdfInput::Path(_)));
        assert_eq!(input.path(), Some(std::path::Path::new("/tmp/test.pdf")));
    }

    #[test]
    fn from_bytes_creates_bytes_variant() {
        let input = PdfInput::from_bytes(vec![1, 2, 3]);
        assert!(matches!(input, PdfInput::Bytes(_)));
        assert!(input.path().is_none());
    }

    #[test]
    fn from_path_with_pathbuf() {
        let pb = std::path::PathBuf::from("/tmp/doc.pdf");
        let input = PdfInput::from_path(pb);
        assert_eq!(input.path(), Some(std::path::Path::new("/tmp/doc.pdf")));
    }

    #[test]
    fn from_bytes_with_slice() {
        let data: Vec<u8> = vec![0x25, 0x50, 0x44, 0x46]; // %PDF
        let input = PdfInput::from_bytes(data.clone());
        if let PdfInput::Bytes(b) = input {
            assert_eq!(b, data);
        } else {
            panic!("expected Bytes variant");
        }
    }

    #[test]
    fn path_returns_none_for_bytes() {
        let input = PdfInput::from_bytes(vec![1, 2]);
        assert!(input.path().is_none());
    }

    #[test]
    fn path_returns_some_for_path() {
        let input = PdfInput::from_path("/tmp/a.pdf");
        assert!(input.path().is_some());
    }

    #[test]
    fn read_bytes_within_limits() {
        let input = PdfInput::from_bytes(vec![1, 2, 3, 4]);
        let limits = ResourceLimits::new();
        let result = input.read(limits);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn read_bytes_exceeds_limits() {
        let data = vec![0u8; 2048];
        let input = PdfInput::from_bytes(data);
        let limits = ResourceLimits::new().with_max_input_bytes(1024);
        let result = input.read(limits);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, PdfError::ResourceLimitExceeded { .. }));
    }

    #[test]
    fn read_path_nonexistent_file() {
        let input = PdfInput::from_path("/nonexistent/path/file.pdf");
        let limits = ResourceLimits::new();
        let result = input.read(limits);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PdfError::Io(_)));
    }

    #[test]
    fn read_path_existing_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_test_input.txt");
        std::fs::write(&path, b"hello pdf").unwrap();
        let input = PdfInput::from_path(&path);
        let limits = ResourceLimits::new();
        let result = input.read(limits);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), b"hello pdf");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn read_path_file_exceeds_limits() {
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_test_large.txt");
        std::fs::write(&path, vec![0u8; 2048]).unwrap();
        let input = PdfInput::from_path(&path);
        let limits = ResourceLimits::new().with_max_input_bytes(1024);
        let result = input.read(limits);
        assert!(result.is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn from_pathbuf_conversion() {
        let pb = std::path::PathBuf::from("/tmp/test.pdf");
        let input: PdfInput = pb.into();
        assert!(matches!(input, PdfInput::Path(_)));
    }

    #[test]
    fn from_path_ref_conversion() {
        let p = std::path::Path::new("/tmp/test.pdf");
        let input: PdfInput = p.into();
        assert!(matches!(input, PdfInput::Path(_)));
    }

    #[test]
    fn from_vec_conversion() {
        let v = vec![1, 2, 3];
        let input: PdfInput = v.into();
        assert!(matches!(input, PdfInput::Bytes(_)));
    }

    #[test]
    fn clone_preserves_variant() {
        let input = PdfInput::from_bytes(vec![1, 2, 3]);
        let cloned = input.clone();
        if let PdfInput::Bytes(b) = cloned {
            assert_eq!(b, vec![1, 2, 3]);
        } else {
            panic!("expected Bytes");
        }
    }

    #[test]
    fn debug_format() {
        let input = PdfInput::from_bytes(vec![1]);
        let dbg = format!("{:?}", input);
        assert!(dbg.contains("Bytes"));
    }
}
