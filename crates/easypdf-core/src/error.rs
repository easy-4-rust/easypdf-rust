//! `easypdf-rust` 的错误类型。
//!
//! 提供核心 `PdfError` 枚举和便捷的 `Result` 类型别名。

use std::io;

/// `easypdf-rust` 的核心错误类型。
///
/// 涵盖 I/O、解析、加密和不支持的功能错误。
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PdfError {
    /// 包装标准 I/O 错误。
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// PDF 无法解析或包含格式错误的数据。
    #[error("PDF parse error: {0}")]
    Parse(String),

    /// 页索引超出范围。
    #[error("Invalid page index: {0}")]
    InvalidPage(usize),

    /// 请求的功能尚未实现或不被引擎支持。
    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),

    /// PDF 已加密且未提供密码或密码错误。
    #[error("Encryption error: {0}")]
    Encryption(String),

    /// 配置的资源限制被超出。
    #[error("Resource limit exceeded for {resource}: actual {actual}, limit {limit}")]
    ResourceLimitExceeded {
        /// 资源名称。
        resource: &'static str,
        /// 配置的限制值。
        limit: u64,
        /// 实际观测值。
        actual: u64,
    },

    /// 安全守卫拒绝了输入（解压炸弹、SSRF 等）。
    #[error("security violation: {0}")]
    SecurityViolation(String),

    /// 数字签名操作失败。
    #[error("signature error: {0}")]
    Signature(String),

    /// 其他错误的兜底变体。
    #[error("{0}")]
    Other(String),
}

impl PdfError {
    /// 返回稳定的机器可读错误码。
    #[must_use]
    pub const fn code(&self) -> PdfErrorCode {
        match self {
            Self::Io(_) => PdfErrorCode::Io,
            Self::Parse(_) => PdfErrorCode::Parse,
            Self::InvalidPage(_) => PdfErrorCode::InvalidPage,
            Self::UnsupportedFeature(_) => PdfErrorCode::UnsupportedFeature,
            Self::Encryption(_) => PdfErrorCode::Encryption,
            Self::ResourceLimitExceeded { .. } => PdfErrorCode::ResourceLimitExceeded,
            Self::SecurityViolation(_) => PdfErrorCode::SecurityViolation,
            Self::Signature(_) => PdfErrorCode::Signature,
            Self::Other(_) => PdfErrorCode::Other,
        }
    }
}

/// 稳定的机器可读 PDF 错误分类。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PdfErrorCode {
    /// 文件或流 I/O 失败。
    Io,
    /// 格式错误或不支持的 PDF 语法。
    Parse,
    /// 无效的页面选择。
    InvalidPage,
    /// 所选后端未实现该功能。
    UnsupportedFeature,
    /// 加密或密码失败。
    Encryption,
    /// 配置的资源限制被超出。
    ResourceLimitExceeded,
    /// 安全守卫拒绝了输入。
    SecurityViolation,
    /// 数字签名失败。
    Signature,
    /// 未分类的失败。
    Other,
}

/// 使用 [`PdfError`] 作为错误变体的便捷 `Result` 类型。
pub type Result<T, E = PdfError> = std::result::Result<T, E>;

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn io_error_display() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file missing");
        let err = PdfError::Io(io_err);
        assert!(format!("{}", err).contains("I/O error"));
        assert!(format!("{}", err).contains("file missing"));
    }

    #[test]
    fn io_error_from_conversion() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
        let err: PdfError = io_err.into();
        assert!(matches!(err, PdfError::Io(_)));
    }

    #[test]
    fn parse_error_display() {
        let err = PdfError::Parse("bad header".to_string());
        assert_eq!(format!("{}", err), "PDF parse error: bad header");
    }

    #[test]
    fn invalid_page_display() {
        let err = PdfError::InvalidPage(42);
        assert_eq!(format!("{}", err), "Invalid page index: 42");
    }

    #[test]
    fn unsupported_feature_display() {
        let err = PdfError::UnsupportedFeature("encryption".to_string());
        assert_eq!(format!("{}", err), "Unsupported feature: encryption");
    }

    #[test]
    fn encryption_error_display() {
        let err = PdfError::Encryption("wrong password".to_string());
        assert_eq!(format!("{}", err), "Encryption error: wrong password");
    }

    #[test]
    fn resource_limit_exceeded_display() {
        let err = PdfError::ResourceLimitExceeded {
            resource: "input_bytes",
            limit: 1024,
            actual: 2048,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("input_bytes"));
        assert!(msg.contains("1024"));
        assert!(msg.contains("2048"));
    }

    #[test]
    fn security_violation_display() {
        let err = PdfError::SecurityViolation("SSRF blocked".to_string());
        assert_eq!(format!("{}", err), "security violation: SSRF blocked");
    }

    #[test]
    fn signature_error_display() {
        let err = PdfError::Signature("invalid cert".to_string());
        assert_eq!(format!("{}", err), "signature error: invalid cert");
    }

    #[test]
    fn other_error_display() {
        let err = PdfError::Other("something".to_string());
        assert_eq!(format!("{}", err), "something");
    }

    // --- error code tests ---

    #[test]
    fn code_io() {
        let err = PdfError::Io(io::Error::other("x"));
        assert_eq!(err.code(), PdfErrorCode::Io);
    }

    #[test]
    fn code_parse() {
        let err = PdfError::Parse("x".to_string());
        assert_eq!(err.code(), PdfErrorCode::Parse);
    }

    #[test]
    fn code_invalid_page() {
        let err = PdfError::InvalidPage(0);
        assert_eq!(err.code(), PdfErrorCode::InvalidPage);
    }

    #[test]
    fn code_unsupported_feature() {
        let err = PdfError::UnsupportedFeature("x".to_string());
        assert_eq!(err.code(), PdfErrorCode::UnsupportedFeature);
    }

    #[test]
    fn code_encryption() {
        let err = PdfError::Encryption("x".to_string());
        assert_eq!(err.code(), PdfErrorCode::Encryption);
    }

    #[test]
    fn code_resource_limit_exceeded() {
        let err = PdfError::ResourceLimitExceeded {
            resource: "pages",
            limit: 100,
            actual: 200,
        };
        assert_eq!(err.code(), PdfErrorCode::ResourceLimitExceeded);
    }

    #[test]
    fn code_security_violation() {
        let err = PdfError::SecurityViolation("x".to_string());
        assert_eq!(err.code(), PdfErrorCode::SecurityViolation);
    }

    #[test]
    fn code_signature() {
        let err = PdfError::Signature("x".to_string());
        assert_eq!(err.code(), PdfErrorCode::Signature);
    }

    #[test]
    fn code_other() {
        let err = PdfError::Other("x".to_string());
        assert_eq!(err.code(), PdfErrorCode::Other);
    }

    // --- PdfErrorCode tests ---

    #[test]
    fn error_code_clone() {
        let code = PdfErrorCode::Io;
        let cloned = code;
        assert_eq!(code, cloned);
    }

    #[test]
    fn error_code_debug() {
        assert_eq!(format!("{:?}", PdfErrorCode::Parse), "Parse");
        assert_eq!(format!("{:?}", PdfErrorCode::Io), "Io");
    }

    #[test]
    fn error_code_eq() {
        assert_eq!(PdfErrorCode::Io, PdfErrorCode::Io);
        assert_ne!(PdfErrorCode::Io, PdfErrorCode::Parse);
    }

    #[test]
    fn error_code_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(PdfErrorCode::Io);
        set.insert(PdfErrorCode::Io);
        set.insert(PdfErrorCode::Parse);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn debug_format_all_variants() {
        let errors = vec![
            PdfError::Parse("p".to_string()),
            PdfError::InvalidPage(0),
            PdfError::UnsupportedFeature("u".to_string()),
            PdfError::Encryption("e".to_string()),
            PdfError::SecurityViolation("s".to_string()),
            PdfError::Signature("s".to_string()),
            PdfError::Other("o".to_string()),
        ];
        for err in errors {
            let _ = format!("{:?}", err);
        }
    }
}
