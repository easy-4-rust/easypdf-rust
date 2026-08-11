//! PDF digital signatures (ISO 32000-1 section 12.8, RFC 5652).
//!
//! Implements PKCS#7/CMS detached `SignedData` signatures with
//! RSA-PKCS#1 v1.5 and SHA-256, including `/ByteRange` computation,
//! X.509 certificate embedding, and full signature verification.

#[path = "sign_der.rs"]
mod der;
#[path = "sign_cms.rs"]
mod cms;
#[path = "sign_pdf.rs"]
mod pdf;

#[cfg(test)]
#[path = "sign_tests.rs"]
mod tests;

// Re-export public API.
pub use pdf::{sign_pdf, verify_pdf_signature};

// ============================================================================
// OID constants
// ============================================================================

/// OID 1.2.840.113549.1.7.2 -- signedData
pub(super) const OID_SIGNED_DATA_DER: &[u8] =
    &[0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x02];
/// OID 1.2.840.113549.1.7.1 -- data
pub(super) const OID_DATA_DER: &[u8] =
    &[0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x01];
/// OID 2.16.840.1.101.3.4.2.1 -- sha-256
pub(super) const OID_SHA256_DER: &[u8] =
    &[0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01];
/// OID 1.2.840.113549.1.1.11 -- sha256WithRSAEncryption
pub(super) const OID_SHA256_WITH_RSA_DER: &[u8] =
    &[0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0B];

// OID value bytes (without tag and length) for comparison during parsing.
pub(super) const OID_SIGNED_DATA_VAL: &[u8] =
    &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x02];

// ============================================================================
// PDF signer configuration
// ============================================================================

/// Configuration for PDF digital signing.
///
/// # Examples
///
/// ```
/// use easypdf_core::crypto::PdfSigner;
///
/// let signer = PdfSigner::new(vec![0u8; 100], vec![0u8; 100])
///     .with_reason("Approval");
/// assert_eq!(signer.reason.as_deref(), Some("Approval"));
/// ```
pub struct PdfSigner {
    /// DER-encoded X.509 certificate, embedded in the PDF signature.
    pub certificate: Vec<u8>,
    /// DER-encoded private key (PKCS#1 or PKCS#8 format).
    pub private_key: Vec<u8>,
    /// Reason for signing (e.g., "Approval", "Reviewed").
    pub reason: Option<String>,
    /// Location of the signer.
    pub location: Option<String>,
    /// Contact information for the signer.
    pub contact_info: Option<String>,
    /// RFC 3161 timestamp server URL (reserved; not yet implemented).
    pub timestamp_url: Option<String>,
}

impl PdfSigner {
    /// Create a new signer with the given X.509 certificate and private key (DER bytes).
    #[must_use]
    pub fn new(certificate: Vec<u8>, private_key: Vec<u8>) -> Self {
        Self {
            certificate,
            private_key,
            reason: None,
            location: None,
            contact_info: None,
            timestamp_url: None,
        }
    }

    /// Set the signing reason.
    #[must_use]
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Set the signing location.
    #[must_use]
    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    /// Set the contact info.
    #[must_use]
    pub fn with_contact_info(mut self, info: impl Into<String>) -> Self {
        self.contact_info = Some(info.into());
        self
    }

    /// Set the RFC 3161 timestamp server URL.
    #[must_use]
    pub fn with_timestamp_url(mut self, url: impl Into<String>) -> Self {
        self.timestamp_url = Some(url.into());
        self
    }
}

// ============================================================================
// Signature information
// ============================================================================

/// Information about a PDF digital signature.
#[derive(Debug, Clone)]
pub struct SignatureInfo {
    /// DER-encoded certificate of the signer.
    pub signer_cert: Vec<u8>,
    /// When the signature was created (PDF date string from /M).
    pub signed_at: Option<String>,
    /// The reason recorded in the signature dictionary.
    pub reason: Option<String>,
    /// The location recorded in the signature dictionary.
    pub location: Option<String>,
    /// Whether the signature is cryptographically valid.
    pub is_valid: bool,
    /// Signer's Common Name (CN) from the X.509 certificate subject.
    pub signer_name: Option<String>,
    /// Issuer distinguished name from the X.509 certificate.
    pub issuer: Option<String>,
    /// Certificate not-before validity (formatted string).
    pub cert_not_before: Option<String>,
    /// Certificate not-after validity (formatted string).
    pub cert_not_after: Option<String>,
}
