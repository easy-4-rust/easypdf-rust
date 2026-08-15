//! PDF 加密算法族。

/// 加密算法族。
///
/// # Examples
///
/// ```
/// use easypdf_core::crypto::PdfEncryptionAlgorithm;
///
/// assert_eq!(PdfEncryptionAlgorithm::Aes128, PdfEncryptionAlgorithm::Aes128);
/// assert_ne!(PdfEncryptionAlgorithm::Aes128, PdfEncryptionAlgorithm::Aes256);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum PdfEncryptionAlgorithm {
    /// AES-128-CBC（`/V 4`、`/R 4`）。128 位文件加密密钥。
    Aes128,
    /// AES-256-CBC（`/V 5`、`/R 6`）。256 位文件加密密钥（ISO 32000-2）。
    Aes256,
}
