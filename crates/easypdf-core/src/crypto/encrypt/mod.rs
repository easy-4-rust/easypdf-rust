//! PDF 加密与解密（ISO 32000-1 第 7.6 节及 ISO 32000-2）。
//!
//! 本模块实现**标准安全处理器**（`/Filter /Standard`），
//! 包含两个算法族：
//!
//! - **AES-128**（`/V 4`、`/R 4`）：AES-128-CBC，128 位文件加密密钥。
//! - **AES-256**（`/V 5`、`/R 6`）：AES-256-CBC，256 位文件加密密钥
//!   （ISO 32000-2）。
//!
//! 核心工作——迭代密钥派生（算法 2 / 2.A / 3 / 4 / 5 / 6）、
//! 逐对象密钥计算（算法 1）、透明字符串/流加密、
//! 以及 `/Encrypt` 字典构建——委托给
//! [`lopdf::Document::encrypt`] 和 [`lopdf::Document::decrypt`]，
//! 它们完整实现了这些规范算法。
//!
//! # 用法
//!
//! ```no_run
//! use easypdf_core::crypto::{
//!     encrypt_pdf, decrypt_pdf, PdfEncryption, PdfEncryptionAlgorithm, PdfPermissions,
//! };
//!
//! let pdf_bytes = std::fs::read("input.pdf").unwrap();
//!
//! // 使用 AES-256 和受限权限加密。
//! let enc = PdfEncryption::new("user", "owner")
//!     .with_algorithm(PdfEncryptionAlgorithm::Aes256)
//!     .with_permissions(PdfPermissions::PRINT | PdfPermissions::COPY);
//! let encrypted = encrypt_pdf(&pdf_bytes, &enc).unwrap();
//!
//! // 使用用户密码解密。
//! let decrypted = decrypt_pdf(&encrypted, "user").unwrap();
//! ```

mod encryption_info;
mod ops;
mod pdf_encryption;
mod pdf_encryption_algorithm;
mod pdf_permissions;

#[cfg(test)]
mod tests;

// 从 ops 子模块重新导出公共函数。
pub use ops::{decrypt_pdf, encrypt_pdf, get_encryption_info};

// 从子模块重新导出公共类型。
pub use encryption_info::EncryptionInfo;
pub use pdf_encryption::PdfEncryption;
pub use pdf_encryption_algorithm::PdfEncryptionAlgorithm;
pub use pdf_permissions::PdfPermissions;
