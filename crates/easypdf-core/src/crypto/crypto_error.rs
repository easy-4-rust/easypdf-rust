//! PDF 加密与签名操作的错误类型。

/// PDF 文档加密操作特有的错误。
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CryptoError {
    /// AES 加密或解密失败。
    #[error("AES error: {0}")]
    Aes(String),

    /// RSA 密钥操作失败。
    #[error("RSA error: {0}")]
    Rsa(String),

    /// RSA 签名创建失败。
    #[error("signature error: {0}")]
    Signature(String),

    /// RSA 签名验证失败。
    #[error("verification error: {0}")]
    Verification(String),

    /// 输入不是本库生成的有效加密 PDF。
    #[error("invalid encrypted PDF: {0}")]
    InvalidEncryptedPdf(String),

    /// 密码不正确。
    #[error("invalid password: {0}")]
    InvalidPassword(String),

    /// 输入不是有效的签名 PDF 或签名格式错误。
    #[error("invalid signed PDF: {0}")]
    InvalidSignedPdf(String),

    /// 无法解析私钥。
    #[error("invalid key format: {0}")]
    InvalidKey(String),

    /// 无法解析证书。
    #[error("invalid certificate: {0}")]
    InvalidCertificate(String),

    /// 发生了 I/O 错误。
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
