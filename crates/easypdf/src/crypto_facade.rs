//! Encrypt and sign facade methods for [`EasyPdf`].

use std::path::Path;

use crate::{EasyPdf, PdfError, Result};

impl EasyPdf {
    /// Encrypt an existing PDF with a password using AES-256-CBC.
    ///
    /// Both `input` and `output` are file paths. The same password is used as
    /// both user and owner password for simplicity.
    ///
    /// **Implementation note**: This uses a simplified encryption container
    /// (not full PDF 2.0 stream-level encryption). See
    /// [`easypdf_core::crypto`] for details on limitations.
    ///
    /// # Errors
    ///
    /// Returns an error if the input file cannot be read, the encryption
    /// fails, or the output cannot be written.
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

    /// Digitally sign a PDF using RSA PKCS#1 v1.5 with SHA-256.
    ///
    /// Reads the RSA private key from the file at the path given in
    /// `private_key_path` (PKCS#1 or PKCS#8 DER format). The signing
    /// certificate is read from `cert_path` (DER-encoded X.509).
    ///
    /// **Implementation note**: This embeds a simplified `/Sig` dictionary,
    /// not a full PKCS#7 `SignedData` container. See [`easypdf_core::crypto`]
    /// for details on limitations.
    ///
    /// # Errors
    ///
    /// Returns an error if the input file, key, or certificate cannot be
    /// read, or if the signing operation fails.
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
        let signer = easypdf_core::crypto::PdfSigner::new(certificate, private_key)
            .with_reason(reason);
        let signed_bytes = easypdf_core::crypto::sign_pdf(&pdf_bytes, &signer)
            .map_err(|e| PdfError::Signature(e.to_string()))?;
        std::fs::write(output, signed_bytes)?;
        Ok(())
    }
}
