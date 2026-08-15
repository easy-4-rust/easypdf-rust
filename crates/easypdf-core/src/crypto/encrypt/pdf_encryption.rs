//! PDF 加密配置。

use super::{PdfEncryptionAlgorithm, PdfPermissions};

/// PDF 加密配置。
///
/// 使用构建器方法 [`with_algorithm`](Self::with_algorithm) 和
/// [`with_permissions`](Self::with_permissions) 进行自定义。默认使用
/// AES-256 并授予所有权限。
///
/// # Examples
///
/// ```
/// use easypdf_core::crypto::{PdfEncryption, PdfEncryptionAlgorithm, PdfPermissions};
///
/// let enc = PdfEncryption::new("user", "owner");
/// assert_eq!(enc.algorithm, PdfEncryptionAlgorithm::Aes256);
///
/// let enc = PdfEncryption::new("u", "o")
///     .with_algorithm(PdfEncryptionAlgorithm::Aes128)
///     .with_permissions(PdfPermissions::PRINT | PdfPermissions::COPY);
/// assert_eq!(enc.algorithm, PdfEncryptionAlgorithm::Aes128);
/// ```
pub struct PdfEncryption {
    /// 打开和读取文档所需的密码。
    pub user_password: String,
    /// 更改权限或移除加密所需的密码。
    pub owner_password: String,
    /// 要使用的加密算法。
    pub algorithm: PdfEncryptionAlgorithm,
    /// 用户密码的权限标志。
    pub permissions: PdfPermissions,
}

impl PdfEncryption {
    /// 创建使用 AES-256 和全部权限的加密配置。
    pub fn new(user: impl Into<String>, owner: impl Into<String>) -> Self {
        Self {
            user_password: user.into(),
            owner_password: owner.into(),
            algorithm: PdfEncryptionAlgorithm::Aes256,
            permissions: PdfPermissions::all(),
        }
    }

    /// 设置加密算法。
    #[must_use]
    pub fn with_algorithm(mut self, algorithm: PdfEncryptionAlgorithm) -> Self {
        self.algorithm = algorithm;
        self
    }

    /// 设置权限标志。
    #[must_use]
    pub fn with_permissions(mut self, permissions: PdfPermissions) -> Self {
        self.permissions = permissions;
        self
    }
}
