//! PDF 数字签名（ISO 32000-1 第 12.8 节，RFC 5652）。
//!
//! 实现 PKCS#7/CMS 分离式 `SignedData` 签名，使用
//! RSA-PKCS#1 v1.5 和 SHA-256，包括 `/ByteRange` 计算、
//! X.509 证书嵌入和完整签名验证。

#[path = "sign_cms.rs"]
mod cms;
#[path = "sign_der.rs"]
mod der;
#[path = "sign_pdf.rs"]
mod pdf;

#[cfg(test)]
#[path = "sign_tests.rs"]
mod tests;

// Re-export public API.
pub use pdf::{sign_pdf, verify_pdf_signature};

// ============================================================================
// OID 常量
// ============================================================================

/// OID 1.2.840.113549.1.7.2——signedData
pub(super) const OID_SIGNED_DATA_DER: &[u8] = &[
    0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x02,
];
/// OID 1.2.840.113549.1.7.1——data
pub(super) const OID_DATA_DER: &[u8] = &[
    0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x01,
];
/// OID 2.16.840.1.101.3.4.2.1——sha-256
pub(super) const OID_SHA256_DER: &[u8] = &[
    0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01,
];
/// OID 1.2.840.113549.1.1.11——sha256WithRSAEncryption
pub(super) const OID_SHA256_WITH_RSA_DER: &[u8] = &[
    0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0B,
];

// OID 值字节（不含标签和长度），用于解析时的比较。
pub(super) const OID_SIGNED_DATA_VAL: &[u8] =
    &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x02];

// ============================================================================
// PDF 签名者配置
// ============================================================================

/// PDF 数字签名配置。
///
/// # Examples
///
/// ```
/// use easypdf_core::crypto::PdfSigner;
///
/// let signer = PdfSigner::new(vec![0u8; 100], vec![0u8; 100])
///     .with_reason("Approval");
/// assert_eq!(signer.reason.as_deref(), Some("Approval"));
/// ```
pub struct PdfSigner {
    /// DER 编码的 X.509 证书，嵌入 PDF 签名中。
    pub certificate: Vec<u8>,
    /// DER 编码的私钥（PKCS#1 或 PKCS#8 格式）。
    pub private_key: Vec<u8>,
    /// 签名原因（如 "Approval"、"Reviewed"）。
    pub reason: Option<String>,
    /// 签名者位置。
    pub location: Option<String>,
    /// 签名者联系信息。
    pub contact_info: Option<String>,
    /// RFC 3161 时间戳服务器 URL（保留；尚未实现）。
    pub timestamp_url: Option<String>,
}

impl PdfSigner {
    /// 使用给定的 X.509 证书和私钥（DER 字节）创建新的签名者。
    #[must_use]
    pub fn new(certificate: Vec<u8>, private_key: Vec<u8>) -> Self {
        Self {
            certificate,
            private_key,
            reason: None,
            location: None,
            contact_info: None,
            timestamp_url: None,
        }
    }

    /// 设置签名原因。
    #[must_use]
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// 设置签名位置。
    #[must_use]
    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    /// 设置联系信息。
    #[must_use]
    pub fn with_contact_info(mut self, info: impl Into<String>) -> Self {
        self.contact_info = Some(info.into());
        self
    }

    /// 设置 RFC 3161 时间戳服务器 URL。
    #[must_use]
    pub fn with_timestamp_url(mut self, url: impl Into<String>) -> Self {
        self.timestamp_url = Some(url.into());
        self
    }
}

// ============================================================================
// 签名信息
// ============================================================================

/// PDF 数字签名的信息。
#[derive(Debug, Clone)]
pub struct SignatureInfo {
    /// 签名者的 DER 编码证书。
    pub signer_cert: Vec<u8>,
    /// 签名创建时间（来自 /M 的 PDF 日期字符串）。
    pub signed_at: Option<String>,
    /// 签名字典中记录的原因。
    pub reason: Option<String>,
    /// 签名字典中记录的位置。
    pub location: Option<String>,
    /// 签名是否在密码学上有效。
    pub is_valid: bool,
    /// 来自 X.509 证书主题的签名者通用名称（CN）。
    pub signer_name: Option<String>,
    /// 来自 X.509 证书的颁发者可分辨名称。
    pub issuer: Option<String>,
    /// 证书有效期起始（格式化字符串）。
    pub cert_not_before: Option<String>,
    /// 证书有效期截止（格式化字符串）。
    pub cert_not_after: Option<String>,
}
