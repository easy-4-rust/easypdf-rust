//! 加密、解密和加密信息操作。

use super::{EncryptionInfo, PdfEncryption, PdfEncryptionAlgorithm};
use crate::crypto::CryptoError;

// ============================================================================
// 加密
// ============================================================================

/// 使用标准安全处理器加密 PDF 字节切片。
///
/// 输入必须是有效的**未加密** PDF。输出是有效的 PDF，
/// 在 trailer 中嵌入 `/Encrypt` 字典，所有字符串和流
/// 按 ISO 32000 透明加密规则加密。
///
/// # Errors
///
/// - 输入无法解析为 PDF 或已加密时返回 `CryptoError::InvalidEncryptedPdf`。
/// - 底层加密失败时返回 `CryptoError::Aes`。
pub fn encrypt_pdf(pdf_bytes: &[u8], encryption: &PdfEncryption) -> Result<Vec<u8>, CryptoError> {
    // 1. 解析 PDF。
    let mut doc = lopdf::Document::load_mem(pdf_bytes)
        .map_err(|e| CryptoError::InvalidEncryptedPdf(format!("failed to parse PDF: {e}")))?;

    // 2. 生成文件加密密钥（V5 需要；V4 忽略）。
    let fek = generate_file_encryption_key(encryption);

    // 3. 构建 lopdf EncryptionVersion。
    let version = build_encryption_version(&doc, encryption, &fek);

    // 4. 派生 EncryptionState（运行所有密钥派生算法）。
    let state = lopdf::EncryptionState::try_from(version)
        .map_err(|e| CryptoError::Aes(format!("encryption state derivation failed: {e}")))?;

    // 5. 透明地原地加密所有对象。
    doc.encrypt(&state)
        .map_err(|e| CryptoError::Aes(format!("encrypt failed: {e}")))?;

    // 6. 序列化回字节。
    let mut buf = Vec::new();
    doc.save_to(&mut buf)
        .map_err(|e| CryptoError::Io(std::io::Error::other(e.to_string())))?;

    Ok(buf)
}

/// 解密使用 [`encrypt_pdf`] 加密的 PDF 字节切片。
///
/// 函数将密码同时作为用户密码和所有者密码尝试。如果匹配任一，
/// PDF 将被完全解密（所有字符串和流恢复为明文）。
///
/// # Errors
///
/// - 输入无法解析为 PDF 或没有 `/Encrypt` 字典时返回
///   `CryptoError::InvalidEncryptedPdf`。
/// - 密码不匹配用户或所有者密码时返回 `CryptoError::InvalidPassword`。
pub fn decrypt_pdf(encrypted_bytes: &[u8], password: &str) -> Result<Vec<u8>, CryptoError> {
    // 1. 解析加密 PDF。
    let mut doc = lopdf::Document::load_mem(encrypted_bytes)
        .map_err(|e| CryptoError::InvalidEncryptedPdf(format!("failed to parse PDF: {e}")))?;

    // 2. 解密（内部尝试用户和所有者密码）。
    doc.decrypt(password).map_err(|e| map_decrypt_error(&e))?;

    // 3. 序列化回字节。
    let mut buf = Vec::new();
    doc.save_to(&mut buf)
        .map_err(|e| CryptoError::Io(std::io::Error::other(e.to_string())))?;

    Ok(buf)
}

/// 在不解密的情况下查询加密 PDF 的加密元数据。
///
/// PDF 未加密时返回 `Ok(None)`。
///
/// # Errors
///
/// PDF 无法解析时返回 `CryptoError::InvalidEncryptedPdf`。
pub fn get_encryption_info(encrypted_bytes: &[u8]) -> Result<Option<EncryptionInfo>, CryptoError> {
    let doc = lopdf::Document::load_mem(encrypted_bytes)
        .map_err(|e| CryptoError::InvalidEncryptedPdf(format!("failed to parse PDF: {e}")))?;

    if !doc.is_encrypted() {
        return Ok(None);
    }

    let enc_dict = doc
        .get_encrypted()
        .map_err(|e| CryptoError::InvalidEncryptedPdf(format!("no /Encrypt dict: {e}")))?;

    let version = enc_dict
        .get(b"V")
        .ok()
        .and_then(|o| o.as_i64().ok())
        .unwrap_or(0);

    let revision = enc_dict
        .get(b"R")
        .ok()
        .and_then(|o| o.as_i64().ok())
        .unwrap_or(0);

    let length = enc_dict
        .get(b"Length")
        .ok()
        .and_then(|o| o.as_i64().ok())
        .and_then(|l| u16::try_from(l).ok());

    let algorithm = match version {
        4 => PdfEncryptionAlgorithm::Aes128,
        5 => PdfEncryptionAlgorithm::Aes256,
        _ => return Ok(None),
    };

    Ok(Some(EncryptionInfo {
        algorithm,
        version,
        revision,
        key_length_bits: length,
    }))
}

// ============================================================================
// 内部辅助函数
// ============================================================================

/// 生成文件加密密钥。
///
/// 对于 AES-256（`/V 5`），这是一个随机的 32 字节密钥，
/// 本身被加密并存储在 `/Encrypt` 字典中。对于 AES-128（`/V 4`），
/// 密钥由 lopdf 从密码 + 文档数据派生，因此我们生成一个
/// 将被忽略的占位符。
fn generate_file_encryption_key(enc: &PdfEncryption) -> [u8; 32] {
    use rand::RngCore;
    match enc.algorithm {
        PdfEncryptionAlgorithm::Aes256 => {
            let mut key = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut key);
            key
        }
        PdfEncryptionAlgorithm::Aes128 => [0u8; 32], // V4 不使用
    }
}

/// 将 lopdf 解密错误映射到我们的 `CryptoError`。
fn map_decrypt_error(e: &lopdf::Error) -> CryptoError {
    match e {
        lopdf::Error::InvalidPassword | lopdf::Error::Decryption(_) => {
            CryptoError::InvalidPassword(e.to_string())
        }
        _ => CryptoError::InvalidEncryptedPdf(format!("decryption failed: {e}")),
    }
}

/// 从我们的 `PdfEncryption` 配置构建 `lopdf::EncryptionVersion`。
fn build_encryption_version<'a>(
    doc: &'a lopdf::Document,
    enc: &'a PdfEncryption,
    file_encryption_key: &'a [u8; 32],
) -> lopdf::EncryptionVersion<'a> {
    let permissions = enc.permissions.to_lopdf();
    let stdcf = b"StdCF".to_vec();

    match enc.algorithm {
        PdfEncryptionAlgorithm::Aes128 => {
            let crypt_filter: std::sync::Arc<dyn lopdf::encryption::crypt_filters::CryptFilter> =
                std::sync::Arc::new(lopdf::encryption::crypt_filters::Aes128CryptFilter);

            lopdf::EncryptionVersion::V4 {
                document: doc,
                encrypt_metadata: true,
                crypt_filters: std::collections::BTreeMap::from([(stdcf.clone(), crypt_filter)]),
                stream_filter: stdcf.clone(),
                string_filter: stdcf,
                owner_password: &enc.owner_password,
                user_password: &enc.user_password,
                permissions,
            }
        }
        PdfEncryptionAlgorithm::Aes256 => {
            let crypt_filter: std::sync::Arc<dyn lopdf::encryption::crypt_filters::CryptFilter> =
                std::sync::Arc::new(lopdf::encryption::crypt_filters::Aes256CryptFilter);

            lopdf::EncryptionVersion::V5 {
                encrypt_metadata: true,
                crypt_filters: std::collections::BTreeMap::from([(stdcf.clone(), crypt_filter)]),
                file_encryption_key,
                stream_filter: stdcf.clone(),
                string_filter: stdcf,
                owner_password: &enc.owner_password,
                user_password: &enc.user_password,
                permissions,
            }
        }
    }
}
