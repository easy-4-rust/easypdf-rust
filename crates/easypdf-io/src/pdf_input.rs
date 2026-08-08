//! PDF 输入来源。

use std::path::{Path, PathBuf};

use easypdf_core::{PdfError, Result};

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
