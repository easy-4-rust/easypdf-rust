//! [`EasyPdf`] 的加密与签名门面方法。

use std::path::Path;

use crate::{EasyPdf, PdfError, Result};

impl EasyPdf {
    /// 使用密码对现有 PDF 进行 AES-256-CBC 加密。
    ///
    /// `input` 和 `output` 均为文件路径。为简便起见，
    /// 同一密码同时用作用户密码和所有者密码。
    ///
    /// **实现说明**：使用简化的加密容器（非完整的 PDF 2.0 流级加密）。
    /// 详见 [`easypdf_core::crypto`] 了解限制。
    ///
    /// # Errors
    ///
    /// 如果输入文件无法读取、加密失败或输出无法写入，返回错误。
    pub fn encrypt(
        input: impl AsRef<Path>,
        output: impl AsRef<Path>,
        password: &str,
    ) -> Result<()> {
        let pdf_bytes = std::fs::read(input)?;
        let encryption = easypdf_core::crypto::PdfEncryption::new(password, password);
        let encrypted = easypdf_core::crypto::encrypt_pdf(&pdf_bytes, &encryption)
            .map_err(|e| PdfError::Encryption(e.to_string()))?;
        std::fs::write(output, encrypted)?;
        Ok(())
    }

    /// 使用 RSA PKCS#1 v1.5 和 SHA-256 对 PDF 进行数字签名。
    ///
    /// 从 `private_key_path` 指定的文件路径读取 RSA 私钥
    /// （PKCS#1 或 PKCS#8 DER 格式）。签名证书从 `cert_path`
    /// 读取（DER 编码的 X.509）。
    ///
    /// **实现说明**：嵌入简化的 `/Sig` 字典，非完整的
    /// PKCS#7 `SignedData` 容器。详见 [`easypdf_core::crypto`] 了解限制。
    ///
    /// # Errors
    ///
    /// 如果输入文件、密钥或证书无法读取，或签名操作失败，返回错误。
    pub fn sign(
        input: impl AsRef<Path>,
        output: impl AsRef<Path>,
        private_key_path: &Path,
        cert_path: &Path,
        reason: &str,
    ) -> Result<()> {
        let pdf_bytes = std::fs::read(input)?;
        let private_key = std::fs::read(private_key_path)?;
        let certificate = std::fs::read(cert_path)?;
        let signer =
            easypdf_core::crypto::PdfSigner::new(certificate, private_key).with_reason(reason);
        let signed_bytes = easypdf_core::crypto::sign_pdf(&pdf_bytes, &signer)
            .map_err(|e| PdfError::Signature(e.to_string()))?;
        std::fs::write(output, signed_bytes)?;
        Ok(())
    }
}
