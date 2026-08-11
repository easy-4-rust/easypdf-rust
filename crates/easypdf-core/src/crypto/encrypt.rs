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

use super::CryptoError;

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
pub fn get_encryption_info(
    encrypted_bytes: &[u8],
) -> Result<Option<EncryptionInfo>, CryptoError> {
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
                crypt_filters: std::collections::BTreeMap::from([(
                    stdcf.clone(),
                    crypt_filter,
                )]),
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
                crypt_filters: std::collections::BTreeMap::from([(
                    stdcf.clone(),
                    crypt_filter,
                )]),
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

// ============================================================================
// Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a minimal but valid single-page PDF in memory using lopdf.
    fn create_test_pdf() -> Vec<u8> {
        let mut doc = lopdf::Document::with_version("1.5");

        let pages_id = doc.new_object_id();

        // Font resource.
        let font_id = doc.add_object(lopdf::dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica"
        });

        let resources_id = doc.add_object(lopdf::dictionary! {
            "Font" => lopdf::dictionary! {
                "F1" => font_id
            }
        });

        // Content stream with a text operation.
        let content = lopdf::Stream::new(
            lopdf::Dictionary::new(),
            b"BT /F1 12 Tf 100 700 Td (Hello World) Tj ET".to_vec(),
        );
        let content_id = doc.add_object(content);

        // Page object.
        let page_id = doc.add_object(lopdf::dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id
        });

        // Pages dictionary.
        doc.objects.insert(
            pages_id,
            lopdf::Object::Dictionary(lopdf::dictionary! {
                "Type" => "Pages",
                "Kids" => vec![lopdf::Object::Reference(page_id)],
                "Count" => 1,
                "Resources" => resources_id,
                "MediaBox" => vec![
                    0.into(), 0.into(), 595.into(), 842.into()
                ]
            }),
        );

        // Catalog.
        let catalog_id = doc.add_object(lopdf::dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id
        });

        // Trailer.
        doc.trailer.set("Root", catalog_id);
        doc.trailer.set(
            "ID",
            lopdf::Object::Array(vec![
                lopdf::Object::string_literal(b"TESTID01"),
                lopdf::Object::string_literal(b"TESTID02"),
            ]),
        );

        let mut buf = Vec::new();
        doc.save_to(&mut buf).expect("save_to must succeed");
        buf
    }

    // --- PdfPermissions ---

    #[test]
    fn permissions_bitflags() {
        let p = PdfPermissions::PRINT | PdfPermissions::COPY | PdfPermissions::FILL_FORMS;
        assert!(p.contains(PdfPermissions::PRINT));
        assert!(p.contains(PdfPermissions::COPY));
        assert!(p.contains(PdfPermissions::FILL_FORMS));
        assert!(!p.contains(PdfPermissions::MODIFY));
        assert!(!p.contains(PdfPermissions::HIGH_QUALITY_PRINT));
    }

    #[test]
    fn permissions_all_contains_every_flag() {
        let all = PdfPermissions::all();
        assert!(all.contains(PdfPermissions::PRINT));
        assert!(all.contains(PdfPermissions::MODIFY));
        assert!(all.contains(PdfPermissions::COPY));
        assert!(all.contains(PdfPermissions::ADD_ANNOTATIONS));
        assert!(all.contains(PdfPermissions::FILL_FORMS));
        assert!(all.contains(PdfPermissions::EXTRACT));
        assert!(all.contains(PdfPermissions::ASSEMBLE));
        assert!(all.contains(PdfPermissions::HIGH_QUALITY_PRINT));
    }

    #[test]
    fn permissions_empty_has_no_flags() {
        let empty = PdfPermissions::empty();
        assert!(!empty.contains(PdfPermissions::PRINT));
        assert!(!empty.contains(PdfPermissions::MODIFY));
    }

    #[test]
    fn permissions_to_lopdf_sets_reserved_bits() {
        let p = PdfPermissions::PRINT;
        let lopdf_p = p.to_lopdf();
        // Bits 6-7 must be set.
        assert_eq!(lopdf_p.bits() & (0b11 << 6), 0b11 << 6);
        // Bits 12-15 must be set.
        assert_eq!(lopdf_p.bits() & (0b1111 << 12), 0b1111 << 12);
    }

    // --- PdfEncryption builder ---

    #[test]
    fn pdf_encryption_defaults() {
        let enc = PdfEncryption::new("u", "o");
        assert_eq!(enc.user_password, "u");
        assert_eq!(enc.owner_password, "o");
        assert_eq!(enc.algorithm, PdfEncryptionAlgorithm::Aes256);
        assert_eq!(enc.permissions, PdfPermissions::all());
    }

    #[test]
    fn pdf_encryption_builder() {
        let enc = PdfEncryption::new("u", "o")
            .with_algorithm(PdfEncryptionAlgorithm::Aes128)
            .with_permissions(PdfPermissions::PRINT | PdfPermissions::COPY);
        assert_eq!(enc.algorithm, PdfEncryptionAlgorithm::Aes128);
        assert_eq!(
            enc.permissions,
            PdfPermissions::PRINT | PdfPermissions::COPY
        );
    }

    // --- PDF creation helper ---

    #[test]
    fn create_test_pdf_produces_valid_bytes() {
        let pdf = create_test_pdf();
        assert!(pdf.len() > 100);
        assert!(pdf.starts_with(b"%PDF"));
        // Should parse back.
        let doc = lopdf::Document::load_mem(&pdf);
        assert!(doc.is_ok());
    }

    // --- AES-128 encrypt/decrypt roundtrip ---

    #[test]
    fn encrypt_decrypt_roundtrip_aes128() {
        let pdf = create_test_pdf();
        let enc = PdfEncryption::new("user123", "owner456")
            .with_algorithm(PdfEncryptionAlgorithm::Aes128);

        let encrypted = encrypt_pdf(&pdf, &enc).unwrap();
        assert_ne!(encrypted, pdf);
        // Encrypted PDF must still start with %PDF.
        assert!(encrypted.starts_with(b"%PDF"));

        let decrypted = decrypt_pdf(&encrypted, "user123").unwrap();
        // After decrypt, re-parse and verify content is accessible.
        let doc = lopdf::Document::load_mem(&decrypted).unwrap();
        // The PDF should no longer be encrypted.
        assert!(!doc.is_encrypted());
    }

    // --- AES-256 encrypt/decrypt roundtrip ---

    #[test]
    fn encrypt_decrypt_roundtrip_aes256() {
        let pdf = create_test_pdf();
        let enc = PdfEncryption::new("user123", "owner456")
            .with_algorithm(PdfEncryptionAlgorithm::Aes256);

        let encrypted = encrypt_pdf(&pdf, &enc).unwrap();
        assert_ne!(encrypted, pdf);
        assert!(encrypted.starts_with(b"%PDF"));

        let decrypted = decrypt_pdf(&encrypted, "user123").unwrap();
        let doc = lopdf::Document::load_mem(&decrypted).unwrap();
        assert!(!doc.is_encrypted());
    }

    // --- Owner password also works for decryption ---

    #[test]
    fn decrypt_with_owner_password() {
        let pdf = create_test_pdf();
        let enc = PdfEncryption::new("user", "supersecret");
        let encrypted = encrypt_pdf(&pdf, &enc).unwrap();

        // Both passwords should work.
        let dec_user = decrypt_pdf(&encrypted, "user");
        assert!(dec_user.is_ok());

        let dec_owner = decrypt_pdf(&encrypted, "supersecret");
        assert!(dec_owner.is_ok());
    }

    // --- Wrong password fails ---

    #[test]
    fn encrypt_wrong_password_fails() {
        let pdf = create_test_pdf();
        let enc = PdfEncryption::new("correct", "owner");
        let encrypted = encrypt_pdf(&pdf, &enc).unwrap();

        let result = decrypt_pdf(&encrypted, "wrong_password");
        assert!(result.is_err());
        match result.unwrap_err() {
            CryptoError::InvalidPassword(_) => {} // expected
            other => panic!("expected InvalidPassword, got: {other:?}"),
        }
    }

    // --- Encrypted PDF has /Encrypt dictionary ---

    #[test]
    fn encrypted_pdf_has_encrypt_dict_aes128() {
        let pdf = create_test_pdf();
        let enc = PdfEncryption::new("u", "o")
            .with_algorithm(PdfEncryptionAlgorithm::Aes128);
        let encrypted = encrypt_pdf(&pdf, &enc).unwrap();

        let doc = lopdf::Document::load_mem(&encrypted).unwrap();
        assert!(doc.is_encrypted());

        let enc_dict = doc.get_encrypted().unwrap();
        assert_eq!(
            enc_dict.get(b"Filter").unwrap().as_name().unwrap(),
            b"Standard"
        );
        assert_eq!(enc_dict.get(b"V").unwrap().as_i64().unwrap(), 4);
        assert_eq!(enc_dict.get(b"R").unwrap().as_i64().unwrap(), 4);
    }

    // --- AES-256 /Encrypt dictionary has V=5, R=6 ---

    #[test]
    fn encrypted_pdf_has_encrypt_dict_aes256() {
        let pdf = create_test_pdf();
        let enc = PdfEncryption::new("u", "o")
            .with_algorithm(PdfEncryptionAlgorithm::Aes256);
        let encrypted = encrypt_pdf(&pdf, &enc).unwrap();

        let doc = lopdf::Document::load_mem(&encrypted).unwrap();
        assert!(doc.is_encrypted());

        let enc_dict = doc.get_encrypted().unwrap();
        assert_eq!(
            enc_dict.get(b"Filter").unwrap().as_name().unwrap(),
            b"Standard"
        );
        assert_eq!(enc_dict.get(b"V").unwrap().as_i64().unwrap(), 5);
        assert_eq!(enc_dict.get(b"R").unwrap().as_i64().unwrap(), 6);
    }

    // --- Permissions are written into /Encrypt dictionary ---

    #[test]
    fn permissions_written_to_encrypt_dict() {
        let pdf = create_test_pdf();
        let perms = PdfPermissions::PRINT | PdfPermissions::COPY;
        let enc = PdfEncryption::new("u", "o")
            .with_permissions(perms)
            .with_algorithm(PdfEncryptionAlgorithm::Aes128);
        let encrypted = encrypt_pdf(&pdf, &enc).unwrap();

        let doc = lopdf::Document::load_mem(&encrypted).unwrap();
        let enc_dict = doc.get_encrypted().unwrap();
        // The /P value is stored as a signed 32-bit integer; truncate to u32 for bit checks.
        let p_raw = enc_dict.get(b"P").unwrap().as_i64().unwrap();
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let p_value = p_raw as u32;

        // The PRINT and COPY bits must be set.
        assert_eq!(p_value & (1 << 2), 1 << 2, "PRINT bit must be set");
        assert_eq!(p_value & (1 << 4), 1 << 4, "COPY bit must be set");
        // MODIFY bit must NOT be set.
        assert_eq!(p_value & (1 << 3), 0, "MODIFY bit must be clear");
    }

    // --- get_encryption_info ---

    #[test]
    fn get_encryption_info_returns_none_for_plain_pdf() {
        let pdf = create_test_pdf();
        let info = get_encryption_info(&pdf).unwrap();
        assert!(info.is_none());
    }

    #[test]
    fn get_encryption_info_returns_correct_algorithm() {
        let pdf = create_test_pdf();

        let enc128 = PdfEncryption::new("u", "o")
            .with_algorithm(PdfEncryptionAlgorithm::Aes128);
        let encrypted = encrypt_pdf(&pdf, &enc128).unwrap();
        let info = get_encryption_info(&encrypted).unwrap().unwrap();
        assert_eq!(info.algorithm, PdfEncryptionAlgorithm::Aes128);
        assert_eq!(info.version, 4);
        assert_eq!(info.revision, 4);

        let enc256 = PdfEncryption::new("u", "o")
            .with_algorithm(PdfEncryptionAlgorithm::Aes256);
        let encrypted = encrypt_pdf(&pdf, &enc256).unwrap();
        let info = get_encryption_info(&encrypted).unwrap().unwrap();
        assert_eq!(info.algorithm, PdfEncryptionAlgorithm::Aes256);
        assert_eq!(info.version, 5);
        assert_eq!(info.revision, 6);
    }

    // --- Encrypted PDF contains /StdCF crypt filter ---

    #[test]
    fn encrypted_pdf_has_std_cf_filter() {
        let pdf = create_test_pdf();
        let enc = PdfEncryption::new("u", "o")
            .with_algorithm(PdfEncryptionAlgorithm::Aes128);
        let encrypted = encrypt_pdf(&pdf, &enc).unwrap();

        let doc = lopdf::Document::load_mem(&encrypted).unwrap();
        let enc_dict = doc.get_encrypted().unwrap();

        // /CF must contain /StdCF.
        let cf = enc_dict.get(b"CF").unwrap().as_dict().unwrap();
        assert!(cf.has(b"StdCF"));

        // /StmF and /StrF must reference /StdCF.
        let stm_f = enc_dict.get(b"StmF").unwrap().as_name().unwrap();
        assert_eq!(stm_f, b"StdCF");
        let str_f = enc_dict.get(b"StrF").unwrap().as_name().unwrap();
        assert_eq!(str_f, b"StdCF");
    }

    // --- Not a PDF ---

    #[test]
    fn encrypt_non_pdf_fails() {
        let result = encrypt_pdf(b"this is not a pdf", &PdfEncryption::new("u", "o"));
        assert!(result.is_err());
    }

    #[test]
    fn decrypt_non_pdf_fails() {
        let result = decrypt_pdf(b"not encrypted", "pass");
        assert!(result.is_err());
    }

    // --- Multi-page roundtrip ---

    #[test]
    fn encrypt_decrypt_multipage_pdf() {
        let mut doc = lopdf::Document::with_version("1.5");
        let pages_id = doc.new_object_id();

        let font_id = doc.add_object(lopdf::dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica"
        });
        let resources_id = doc.add_object(lopdf::dictionary! {
            "Font" => lopdf::dictionary! { "F1" => font_id }
        });

        let mut page_refs = Vec::new();
        for i in 0..3 {
            let content = lopdf::Stream::new(
                lopdf::Dictionary::new(),
                format!("BT /F1 12 Tf 100 700 Td (Page {i}) Tj ET").into_bytes(),
            );
            let content_id = doc.add_object(content);
            let page_id = doc.add_object(lopdf::dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Contents" => content_id
            });
            page_refs.push(lopdf::Object::Reference(page_id));
        }

        doc.objects.insert(
            pages_id,
            lopdf::Object::Dictionary(lopdf::dictionary! {
                "Type" => "Pages",
                "Kids" => page_refs,
                "Count" => 3,
                "Resources" => resources_id,
                "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()]
            }),
        );

        let catalog_id = doc.add_object(lopdf::dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id
        });
        doc.trailer.set("Root", catalog_id);
        doc.trailer.set(
            "ID",
            lopdf::Object::Array(vec![
                lopdf::Object::string_literal(b"MULTIPAGE1"),
                lopdf::Object::string_literal(b"MULTIPAGE2"),
            ]),
        );

        let mut pdf_bytes = Vec::new();
        doc.save_to(&mut pdf_bytes).unwrap();

        let enc = PdfEncryption::new("pass", "owner")
            .with_algorithm(PdfEncryptionAlgorithm::Aes128)
            .with_permissions(PdfPermissions::all());

        let encrypted = encrypt_pdf(&pdf_bytes, &enc).unwrap();
        let decrypted = decrypt_pdf(&encrypted, "pass").unwrap();

        // Verify we can parse the decrypted result.
        let doc = lopdf::Document::load_mem(&decrypted).unwrap();
        assert!(!doc.is_encrypted());
        // Verify the trailer has a /Root (catalog) reference.
        assert!(
            doc.trailer.has(b"Root"),
            "trailer must have /Root after multipage roundtrip"
        );
    }

    // --- Encrypt then decrypt preserves page content ---

    #[test]
    fn roundtrip_preserves_document_structure() {
        let pdf = create_test_pdf();
        let enc = PdfEncryption::new("u", "o");
        let encrypted = encrypt_pdf(&pdf, &enc).unwrap();
        let decrypted = decrypt_pdf(&encrypted, "u").unwrap();

        // The decrypted output must be valid PDF bytes.
        assert!(decrypted.starts_with(b"%PDF"));
        // Re-load: must parse without error and be unencrypted.
        let doc = lopdf::Document::load_mem(&decrypted).unwrap();
        assert!(!doc.is_encrypted());
        // Trailer must still have /Root.
        assert!(
            doc.trailer.has(b"Root"),
            "trailer must have /Root after roundtrip"
        );
    }
}
