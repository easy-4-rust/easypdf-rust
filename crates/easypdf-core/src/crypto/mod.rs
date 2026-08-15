//! PDF encryption and digital signature operations (ISO 32000).
//!
//! This module is split into two submodules:
//!
//! - [`encrypt`]: Password-based encryption (`/V 4`, `/R 4` AES-128 / `/V 5` AES-256).
//! - [`sign`]: Digital signatures (`PKCS#7` detached `SignedData`, RSA-PKCS#1v1.5).
//!
//! Both submodules aim for PDF spec (ISO 32000-1 / ISO 32000-2) compliance.

pub mod encrypt;
pub mod sign;

pub use encrypt::{
    EncryptionInfo, PdfEncryption, PdfEncryptionAlgorithm, PdfPermissions, decrypt_pdf,
    encrypt_pdf, get_encryption_info,
};
pub use sign::{PdfSigner, SignatureInfo, sign_pdf, verify_pdf_signature};

// ============================================================================
// Error type
// ============================================================================

/// Errors specific to cryptographic operations on PDF documents.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CryptoError {
    /// AES encryption or decryption failed.
    #[error("AES error: {0}")]
    Aes(String),

    /// RSA key operation failed.
    #[error("RSA error: {0}")]
    Rsa(String),

    /// RSA signature creation failed.
    #[error("signature error: {0}")]
    Signature(String),

    /// RSA signature verification failed.
    #[error("verification error: {0}")]
    Verification(String),

    /// The input is not a valid encrypted PDF produced by this library.
    #[error("invalid encrypted PDF: {0}")]
    InvalidEncryptedPdf(String),

    /// The password is incorrect for the encrypted PDF.
    #[error("invalid password: {0}")]
    InvalidPassword(String),

    /// The input is not a valid signed PDF or the signature is malformed.
    #[error("invalid signed PDF: {0}")]
    InvalidSignedPdf(String),

    /// The private key could not be parsed.
    #[error("invalid key format: {0}")]
    InvalidKey(String),

    /// The certificate could not be parsed.
    #[error("invalid certificate: {0}")]
    InvalidCertificate(String),

    /// An I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
