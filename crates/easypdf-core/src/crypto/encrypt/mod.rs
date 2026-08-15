//! PDF encryption and decryption per ISO 32000-1 (section 7.6) and ISO 32000-2.
//!
//! This module implements the **Standard Security Handler** (`/Filter /Standard`)
//! with two algorithm families:
//!
//! - **AES-128** (`/V 4`, `/R 4`): AES-128-CBC with 128-bit file encryption key.
//! - **AES-256** (`/V 5`, `/R 6`): AES-256-CBC with 256-bit file encryption key
//!   (ISO 32000-2).
//!
//! The heavy lifting -- iterative key derivation (Algorithms 2 / 2.A / 3 / 4 / 5 / 6),
//! per-object key computation (Algorithm 1), transparent string/stream encryption,
//! and `/Encrypt` dictionary construction -- is delegated to
//! [`lopdf::Document::encrypt`] and [`lopdf::Document::decrypt`], which implement
//! these algorithms in full spec compliance.
//!
//! # Usage
//!
//! ```no_run
//! use easypdf_core::crypto::{
//!     encrypt_pdf, decrypt_pdf, PdfEncryption, PdfEncryptionAlgorithm, PdfPermissions,
//! };
//!
//! let pdf_bytes = std::fs::read("input.pdf").unwrap();
//!
//! // Encrypt with AES-256 and restricted permissions.
//! let enc = PdfEncryption::new("user", "owner")
//!     .with_algorithm(PdfEncryptionAlgorithm::Aes256)
//!     .with_permissions(PdfPermissions::PRINT | PdfPermissions::COPY);
//! let encrypted = encrypt_pdf(&pdf_bytes, &enc).unwrap();
//!
//! // Decrypt with the user password.
//! let decrypted = decrypt_pdf(&encrypted, "user").unwrap();
//! ```

mod ops;

#[cfg(test)]
mod tests;

// Re-export public functions from the ops submodule.
pub use ops::{decrypt_pdf, encrypt_pdf, get_encryption_info};

// ============================================================================
// Public types
// ============================================================================

/// Permission flags for an encrypted PDF, as defined in ISO 32000-1 table 22.
///
/// These control what operations a user with the *user password* may perform.
/// The owner password bypasses all permission checks.
///
/// # Examples
///
/// ```
/// use easypdf_core::crypto::PdfPermissions;
///
/// let perms = PdfPermissions::PRINT | PdfPermissions::COPY;
/// assert!(perms.contains(PdfPermissions::PRINT));
/// assert!(!perms.contains(PdfPermissions::MODIFY));
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PdfPermissions(u32);

bitflags::bitflags! {
    impl PdfPermissions: u32 {
        /// Print the document.
        const PRINT = 1 << 2;
        /// Modify contents (except as controlled by other flags).
        const MODIFY = 1 << 3;
        /// Copy or extract text and graphics.
        const COPY = 1 << 4;
        /// Add or modify text annotations and fill interactive form fields.
        const ADD_ANNOTATIONS = 1 << 5;
        /// Fill in existing interactive form fields.
        const FILL_FORMS = 1 << 8;
        /// Extract text for accessibility purposes.
        const EXTRACT = 1 << 9;
        /// Assemble the document (insert, rotate, delete pages).
        const ASSEMBLE = 1 << 10;
        /// Print in high quality.
        const HIGH_QUALITY_PRINT = 1 << 11;
    }
}

impl PdfPermissions {
    /// Convert to the lopdf permission type with reserved bits corrected per spec.
    fn to_lopdf(self) -> lopdf::Permissions {
        let mut bits: u64 = u64::from(self.bits());
        // PDF spec requires certain reserved bits to be set to 1.
        bits |= 0b11 << 6;
        bits |= 0b1111 << 12;
        bits |= 0xFFFF << 16;
        bits |= 0xFFFF_FFFF << 32;
        lopdf::Permissions::from_bits_retain(bits)
    }
}

/// The encryption algorithm family.
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
    /// AES-128-CBC (`/V 4`, `/R 4`). 128-bit file encryption key.
    Aes128,
    /// AES-256-CBC (`/V 5`, `/R 6`). 256-bit file encryption key (ISO 32000-2).
    Aes256,
}

/// Configuration for PDF encryption.
///
/// Use the builder methods [`with_algorithm`](Self::with_algorithm) and
/// [`with_permissions`](Self::with_permissions) to customise. The default
/// uses AES-256 with all permissions granted.
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
    /// Password required to open and read the document.
    pub user_password: String,
    /// Password required to change permissions or remove encryption.
    pub owner_password: String,
    /// The encryption algorithm to use.
    pub algorithm: PdfEncryptionAlgorithm,
    /// Permission flags for the user password.
    pub permissions: PdfPermissions,
}

impl PdfEncryption {
    /// Create a new encryption configuration with AES-256 and all permissions.
    pub fn new(user: impl Into<String>, owner: impl Into<String>) -> Self {
        Self {
            user_password: user.into(),
            owner_password: owner.into(),
            algorithm: PdfEncryptionAlgorithm::Aes256,
            permissions: PdfPermissions::all(),
        }
    }

    /// Set the encryption algorithm.
    #[must_use]
    pub fn with_algorithm(mut self, algorithm: PdfEncryptionAlgorithm) -> Self {
        self.algorithm = algorithm;
        self
    }

    /// Set the permission flags.
    #[must_use]
    pub fn with_permissions(mut self, permissions: PdfPermissions) -> Self {
        self.permissions = permissions;
        self
    }
}

/// Metadata about the encryption scheme used in an encrypted PDF.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncryptionInfo {
    /// The encryption algorithm family.
    pub algorithm: PdfEncryptionAlgorithm,
    /// The `/V` value from the Encrypt dictionary.
    pub version: i64,
    /// The `/R` value from the Encrypt dictionary.
    pub revision: i64,
    /// The `/Length` value in bits, if present.
    pub key_length_bits: Option<u16>,
}
