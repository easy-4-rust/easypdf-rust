//! PDF 加密与数字签名操作（ISO 32000）。
//!
//! 本模块分为两个子模块：
//!
//! - [`encrypt`]: 基于密码的加密（`/V 4`、`/R 4` AES-128 / `/V 5` AES-256）。
//! - [`sign`]: 数字签名（`PKCS#7` 分离式 `SignedData`，RSA-PKCS#1v1.5）。
//!
//! 两个子模块均致力于符合 PDF 规范（ISO 32000-1 / ISO 32000-2）。

mod crypto_error;
pub mod encrypt;
pub mod sign;

pub use crypto_error::CryptoError;
pub use encrypt::{
    EncryptionInfo, PdfEncryption, PdfEncryptionAlgorithm, PdfPermissions, decrypt_pdf,
    encrypt_pdf, get_encryption_info,
};
pub use sign::{PdfSigner, SignatureInfo, sign_pdf, verify_pdf_signature};
