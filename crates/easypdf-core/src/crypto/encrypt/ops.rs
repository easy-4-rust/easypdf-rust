//! Encryption, decryption, and encryption-info operations.

use super::{EncryptionInfo, PdfEncryption, PdfEncryptionAlgorithm};
use crate::crypto::CryptoError;

// ============================================================================
// Encryption
// ============================================================================

/// Encrypt a PDF byte slice using the Standard Security Handler.
///
/// The input must be a valid, **unencrypted** PDF. The output is a valid PDF
/// with the `/Encrypt` dictionary embedded in the trailer and all strings and
/// streams encrypted per ISO 32000 transparent encryption rules.
///
/// # Errors
///
/// - `CryptoError::InvalidEncryptedPdf` if the input cannot be parsed as PDF
///   or is already encrypted.
/// - `CryptoError::Aes` if the underlying encryption fails.
pub fn encrypt_pdf(pdf_bytes: &[u8], encryption: &PdfEncryption) -> Result<Vec<u8>, CryptoError> {
    // 1. Parse the PDF.
    let mut doc = lopdf::Document::load_mem(pdf_bytes)
        .map_err(|e| CryptoError::InvalidEncryptedPdf(format!("failed to parse PDF: {e}")))?;

    // 2. Generate the file encryption key (needed for V5; V4 ignores it).
    let fek = generate_file_encryption_key(encryption);

    // 3. Build the lopdf EncryptionVersion.
    let version = build_encryption_version(&doc, encryption, &fek);

    // 4. Derive the EncryptionState (runs all key-derivation algorithms).
    let state = lopdf::EncryptionState::try_from(version)
        .map_err(|e| CryptoError::Aes(format!("encryption state derivation failed: {e}")))?;

    // 5. Transparently encrypt all objects in-place.
    doc.encrypt(&state)
        .map_err(|e| CryptoError::Aes(format!("encrypt failed: {e}")))?;

    // 6. Serialize back to bytes.
    let mut buf = Vec::new();
    doc.save_to(&mut buf)
        .map_err(|e| CryptoError::Io(std::io::Error::other(e.to_string())))?;

    Ok(buf)
}

/// Decrypt a PDF byte slice that was encrypted with [`encrypt_pdf`].
///
/// The function tries the password as both the user and owner password. If it
/// matches either, the PDF is fully decrypted (all strings and streams restored
/// to plaintext).
///
/// # Errors
///
/// - `CryptoError::InvalidEncryptedPdf` if the input cannot be parsed as PDF
///   or has no `/Encrypt` dictionary.
/// - `CryptoError::InvalidPassword` if the password does not match either the
///   user or owner password.
pub fn decrypt_pdf(encrypted_bytes: &[u8], password: &str) -> Result<Vec<u8>, CryptoError> {
    // 1. Parse the encrypted PDF.
    let mut doc = lopdf::Document::load_mem(encrypted_bytes)
        .map_err(|e| CryptoError::InvalidEncryptedPdf(format!("failed to parse PDF: {e}")))?;

    // 2. Decrypt (tries user and owner passwords internally).
    doc.decrypt(password).map_err(|e| map_decrypt_error(&e))?;

    // 3. Serialize back to bytes.
    let mut buf = Vec::new();
    doc.save_to(&mut buf)
        .map_err(|e| CryptoError::Io(std::io::Error::other(e.to_string())))?;

    Ok(buf)
}

/// Query the encryption metadata of an encrypted PDF without decrypting it.
///
/// Returns `Ok(None)` if the PDF is not encrypted.
///
/// # Errors
///
/// Returns `CryptoError::InvalidEncryptedPdf` if the PDF cannot be parsed.
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
// Internal helpers
// ============================================================================

/// Generate the file encryption key.
///
/// For AES-256 (`/V 5`) this is a random 32-byte key that is itself encrypted
/// and stored in the `/Encrypt` dictionary. For AES-128 (`/V 4`) the key is
/// derived by lopdf from passwords + document data, so we generate a placeholder
/// that will be ignored.
fn generate_file_encryption_key(enc: &PdfEncryption) -> [u8; 32] {
    use rand::RngCore;
    match enc.algorithm {
        PdfEncryptionAlgorithm::Aes256 => {
            let mut key = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut key);
            key
        }
        PdfEncryptionAlgorithm::Aes128 => [0u8; 32], // unused for V4
    }
}

/// Map a lopdf decryption error to our `CryptoError`.
fn map_decrypt_error(e: &lopdf::Error) -> CryptoError {
    match e {
        lopdf::Error::InvalidPassword | lopdf::Error::Decryption(_) => {
            CryptoError::InvalidPassword(e.to_string())
        }
        _ => CryptoError::InvalidEncryptedPdf(format!("decryption failed: {e}")),
    }
}

/// Build an `lopdf::EncryptionVersion` from our `PdfEncryption` config.
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
