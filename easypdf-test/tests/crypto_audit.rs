//! Integration tests for PDF encryption and digital signature operations.
//!
//! These tests exercise the `easypdf_core::crypto` module through both the
//! `EasyPdf` facade and direct function calls. They verify:
#![allow(clippy::similar_names)]
//!
//! - AES-256-CBC encrypt/decrypt roundtrip on real PDF files
//! - RSA PKCS#1 v1.5 sign/verify roundtrip on real PDF files
//! - Error handling for wrong passwords, missing files, and invalid inputs
//! - The encrypted output is a standard encrypted PDF (not a custom container)

use easypdf::EasyPdf;
use easypdf_core::crypto::{
    self, CryptoError, PdfEncryption, PdfSigner,
};

// ============================================================================
// DER encoding helpers (for building self-signed X.509 test certificates)
// ============================================================================

/// Encode a DER length field.
#[allow(clippy::cast_possible_truncation)]
fn der_len(len: usize) -> Vec<u8> {
    if len < 0x80 {
        vec![len as u8]
    } else if len < 0x100 {
        vec![0x81, len as u8]
    } else if len < 0x10_000 {
        vec![0x82, (len >> 8) as u8, len as u8]
    } else {
        vec![0x83, (len >> 16) as u8, (len >> 8) as u8, len as u8]
    }
}

/// Wrap a value in a DER TLV (tag-length-value).
fn der_tlv(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(1 + 4 + value.len());
    out.push(tag);
    out.extend(der_len(value.len()));
    out.extend(value);
    out
}

/// DER SEQUENCE wrapper.
fn der_seq(value: &[u8]) -> Vec<u8> {
    der_tlv(0x30, value)
}

/// DER SET wrapper.
fn der_set(value: &[u8]) -> Vec<u8> {
    der_tlv(0x31, value)
}

/// DER INTEGER wrapper. Handles leading-zero for positive integers with MSB=1.
fn der_int(value: &[u8]) -> Vec<u8> {
    let mut content = Vec::with_capacity(value.len() + 1);
    if value.is_empty() {
        content.push(0);
    } else if value[0] & 0x80 != 0 {
        content.push(0);
        content.extend(value);
    } else {
        let mut start = 0;
        while start < value.len() - 1
            && value[start] == 0
            && value[start + 1] & 0x80 == 0
        {
            start += 1;
        }
        content.extend(&value[start..]);
    }
    der_tlv(0x02, &content)
}

/// DER OCTET STRING wrapper.
#[allow(dead_code)]
fn der_octets(value: &[u8]) -> Vec<u8> {
    der_tlv(0x04, value)
}

/// DER BIT STRING wrapper (with 0 unused bits).
fn der_bits(value: &[u8]) -> Vec<u8> {
    let mut content = Vec::with_capacity(1 + value.len());
    content.push(0);
    content.extend(value);
    der_tlv(0x03, &content)
}

/// DER context-specific tag wrapper.
fn der_ctx(tag: u8, constructed: bool, value: &[u8]) -> Vec<u8> {
    let tag_byte = if constructed { 0xA0 | tag } else { 0x80 | tag };
    der_tlv(tag_byte, value)
}

/// DER NULL.
fn der_null() -> Vec<u8> {
    vec![0x05, 0x00]
}

/// DER `UTF8String`.
fn der_utf8(s: &str) -> Vec<u8> {
    der_tlv(0x0C, s.as_bytes())
}

/// DER `UTCTime`.
fn der_utc(s: &str) -> Vec<u8> {
    der_tlv(0x17, s.as_bytes())
}

/// Concatenate byte slices.
fn concat(parts: &[&[u8]]) -> Vec<u8> {
    let len: usize = parts.iter().map(|p| p.len()).sum();
    let mut out = Vec::with_capacity(len);
    for part in parts {
        out.extend_from_slice(part);
    }
    out
}

// OID value constants (pre-DER-encoded: tag 0x06 + length + value).
const OID_RSA_ENCRYPTION_DER: &[u8] =
    &[0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x01];
const OID_SHA256_WITH_RSA_DER: &[u8] =
    &[0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0B];
const OID_CN_DER: &[u8] = &[0x06, 0x03, 0x55, 0x04, 0x03];

// ============================================================================
// Test certificate generation (self-signed X.509 v3)
// ============================================================================

/// Generate a self-signed X.509 certificate and RSA private key for testing.
///
/// Returns `(certificate_der, private_key_der)` where the certificate
/// is a valid DER-encoded X.509 v3 certificate with CN=easypdf-test.
fn generate_test_cert_and_key() -> (Vec<u8>, Vec<u8>) {
    use rand::rngs::OsRng;
    use rsa::pkcs1::EncodeRsaPrivateKey;
    use rsa::RsaPrivateKey;

    let mut rng = OsRng;
    let private_key = RsaPrivateKey::new(&mut rng, 2048).expect("generate RSA key");
    let public_key = private_key.to_public_key();

    let priv_der = private_key
        .to_pkcs1_der()
        .expect("encode private key")
        .as_bytes()
        .to_vec();

    let cert_der = build_self_signed_cert(&priv_der, &public_key);

    (cert_der, priv_der)
}

/// Build a minimal self-signed X.509 v3 certificate in DER format.
///
/// Uses ring for constant-time RSA PKCS#1 v1.5 + SHA-256 signing.
fn build_self_signed_cert(
    priv_der: &[u8],
    public_key: &rsa::RsaPublicKey,
) -> Vec<u8> {
    use ring::rand::SystemRandom;
    use ring::signature::{RsaKeyPair, RSA_PKCS1_SHA256};
    use rsa::traits::PublicKeyParts;

    // Subject/issuer Name: SEQUENCE { SET { SEQUENCE { OID CN, UTF8String } } }
    let cn_attr = der_seq(&concat(&[OID_CN_DER, &der_utf8("easypdf-test")]));
    let name = der_seq(&der_set(&cn_attr));

    // Validity: SEQUENCE { UTCTime, UTCTime }
    let validity = der_seq(&concat(&[
        &der_utc("200101000000Z"),
        &der_utc("491231235959Z"),
    ]));

    // SubjectPublicKeyInfo
    let n_bytes = public_key.n().to_bytes_be();
    let e_bytes = public_key.e().to_bytes_be();
    let rsa_pub_key = der_seq(&concat(&[&der_int(&n_bytes), &der_int(&e_bytes)]));
    let spki = der_seq(&concat(&[
        &der_seq(&concat(&[OID_RSA_ENCRYPTION_DER, &der_null()])),
        &der_bits(&rsa_pub_key),
    ]));

    // TBSCertificate SEQUENCE
    let tbs = der_seq(&concat(&[
        &der_ctx(0, true, &der_int(&[2u8])), // version v3
        &der_int(&[0x01u8]),                   // serial = 1
        &der_seq(OID_SHA256_WITH_RSA_DER),     // signature algorithm
        &name,                                  // issuer
        &validity,                              // validity
        &name,                                  // subject (same as issuer)
        &spki,                                  // subjectPublicKeyInfo
    ]));

    // Sign the TBS certificate with ring (constant-time RSA).
    let key_pair = RsaKeyPair::from_der(priv_der).expect("ring key parse");
    let rng = SystemRandom::new();
    let mut sig_bytes = vec![0u8; key_pair.public().modulus_len()];
    key_pair
        .sign(&RSA_PKCS1_SHA256, &rng, &tbs, &mut sig_bytes)
        .expect("sign cert TBS");

    // Certificate SEQUENCE
    der_seq(&concat(&[
        &tbs,
        &der_seq(OID_SHA256_WITH_RSA_DER),
        &der_bits(&sig_bytes),
    ]))
}

// ============================================================================
// Test PDF generation helpers
// ============================================================================

/// Create a minimal valid PDF in memory using the `PdfWriter` facade.
fn create_test_pdf_bytes() -> Vec<u8> {
    use easypdf::PdfWriter;
    use easypdf::{PageSize, Orientation, PdfText};

    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("test.pdf");
    let mut writer = PdfWriter::new("Test Document");
    writer
        .add_page(PageSize::A4, Orientation::Portrait)
        .expect("add page");
    writer
        .write_text(&PdfText::new("Hello, crypto world!"), 100.0, 700.0)
        .expect("write text");
    writer.finish(&path).expect("finish PDF");
    std::fs::read(&path).expect("read PDF")
}

/// Create a valid PDF with custom text content using lopdf directly.
///
/// This is used for encryption tests that need multiple distinct PDFs
/// to verify that different inputs produce different ciphertexts.
fn make_test_pdf(text: &str) -> Vec<u8> {
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

    let content = lopdf::Stream::new(
        lopdf::Dictionary::new(),
        format!("BT /F1 12 Tf 100 700 Td ({text}) Tj ET").into_bytes(),
    );
    let content_id = doc.add_object(content);

    let page_id = doc.add_object(lopdf::dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content_id
    });

    doc.objects.insert(
        pages_id,
        lopdf::Object::Dictionary(lopdf::dictionary! {
            "Type" => "Pages",
            "Kids" => vec![lopdf::Object::Reference(page_id)],
            "Count" => 1,
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
            lopdf::Object::string_literal(b"TESTID01"),
            lopdf::Object::string_literal(b"TESTID02"),
        ]),
    );

    let mut buf = Vec::new();
    doc.save_to(&mut buf).expect("save_to must succeed");
    buf
}

/// Create a large valid PDF (multi-page with repeated text).
fn make_large_test_pdf() -> Vec<u8> {
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
    for i in 0..50 {
        let content = lopdf::Stream::new(
            lopdf::Dictionary::new(),
            format!(
                "BT /F1 12 Tf 100 700 Td (Page {i} - Lorem ipsum dolor sit amet, \
                 consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore \
                 et dolore magna aliqua. Ut enim ad minim veniam.) Tj ET"
            )
            .into_bytes(),
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
            "Count" => 50,
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
            lopdf::Object::string_literal(b"LARGEID01"),
            lopdf::Object::string_literal(b"LARGEID02"),
        ]),
    );

    let mut buf = Vec::new();
    doc.save_to(&mut buf).expect("save_to must succeed");
    buf
}

// ============================================================================
// Encryption integration tests
// ============================================================================

#[test]
fn encrypt_decrypt_roundtrip_preserves_content() {
    let pdf_bytes = create_test_pdf_bytes();
    let encryption = PdfEncryption::new("userpass", "ownerpass");

    let encrypted = crypto::encrypt_pdf(&pdf_bytes, &encryption).expect("encrypt");

    // Encrypted data is different from original.
    assert_ne!(encrypted, pdf_bytes);
    // Encrypted output is a valid PDF (starts with %PDF).
    assert!(encrypted.starts_with(b"%PDF-"));
    // Encrypted output contains the /Encrypt dictionary.
    let s = String::from_utf8_lossy(&encrypted);
    assert!(s.contains("/Encrypt"), "encrypted PDF must contain /Encrypt dictionary");

    // Decrypt with the owner password.
    let decrypted = crypto::decrypt_pdf(&encrypted, "ownerpass").expect("decrypt");
    // After decrypt, re-parse and verify it is no longer encrypted.
    let doc = lopdf::Document::load_mem(&decrypted).expect("decrypted PDF must parse");
    assert!(!doc.is_encrypted(), "decrypted PDF must not be encrypted");
}

#[test]
fn encrypt_decrypt_wrong_password_fails() {
    let pdf_bytes = create_test_pdf_bytes();
    let encryption = PdfEncryption::new("user", "correct");

    let encrypted = crypto::encrypt_pdf(&pdf_bytes, &encryption).expect("encrypt");
    let result = crypto::decrypt_pdf(&encrypted, "wrong_password");

    assert!(result.is_err());
    match result.unwrap_err() {
        CryptoError::InvalidPassword(_) => {} // Expected: wrong password.
        other => panic!("expected InvalidPassword error, got: {other}"),
    }
}

#[test]
fn encrypt_different_plaintexts_produce_different_ciphertexts() {
    let enc = PdfEncryption::new("u", "o");
    let pdf_a = make_test_pdf("content A");
    let pdf_b = make_test_pdf("content B");
    let e1 = crypto::encrypt_pdf(&pdf_a, &enc).expect("encrypt A");
    let e2 = crypto::encrypt_pdf(&pdf_b, &enc).expect("encrypt B");
    assert_ne!(e1, e2);
}

#[test]
fn encrypt_same_plaintext_produces_different_ciphertexts() {
    // Random IV and salt mean two encryptions of the same data differ.
    let enc = PdfEncryption::new("u", "o");
    let pdf = make_test_pdf("same data");
    let e1 = crypto::encrypt_pdf(&pdf, &enc).expect("encrypt 1");
    let e2 = crypto::encrypt_pdf(&pdf, &enc).expect("encrypt 2");
    assert_ne!(e1, e2);
}

#[test]
fn decrypt_non_encrypted_data_fails() {
    // Non-PDF bytes should fail to parse.
    let result = crypto::decrypt_pdf(b"this is not encrypted", "pass");
    assert!(result.is_err());
    match result.unwrap_err() {
        CryptoError::InvalidEncryptedPdf(_) => {} // Expected: parse failure.
        other => panic!("expected InvalidEncryptedPdf error, got: {other}"),
    }
}

#[test]
fn decrypt_too_short_data_fails() {
    let result = crypto::decrypt_pdf(b"short", "pass");
    assert!(result.is_err());
}

#[test]
fn encrypt_decrypt_empty_pdf() {
    // Empty bytes are not a valid PDF; encrypt should fail.
    let enc = PdfEncryption::new("u", "o");
    let result = crypto::encrypt_pdf(b"", &enc);
    assert!(result.is_err(), "encrypting empty bytes should fail");
}

#[test]
fn encrypt_decrypt_large_pdf() {
    let large_pdf = make_large_test_pdf();
    let enc = PdfEncryption::new("u", "o");
    let encrypted = crypto::encrypt_pdf(&large_pdf, &enc).expect("encrypt large");
    assert!(encrypted.starts_with(b"%PDF-"));
    let decrypted = crypto::decrypt_pdf(&encrypted, "o").expect("decrypt large");
    let doc = lopdf::Document::load_mem(&decrypted).expect("decrypted large PDF must parse");
    assert!(!doc.is_encrypted());
}

// ============================================================================
// Signing integration tests
// ============================================================================

#[test]
fn sign_pdf_produces_signature_dictionary() {
    let pdf_bytes = create_test_pdf_bytes();
    let (cert, priv_key) = generate_test_cert_and_key();

    let signer = PdfSigner::new(cert, priv_key).with_reason("Integration test");
    let signed = crypto::sign_pdf(&pdf_bytes, &signer).expect("sign");

    // Signed PDF is larger than original (signature dict + trailer).
    assert!(signed.len() > pdf_bytes.len());

    // Contains signature dictionary markers.
    let signed_str = String::from_utf8_lossy(&signed);
    assert!(signed_str.contains("/Type /Sig"));
    assert!(signed_str.contains("/ByteRange"));
    assert!(signed_str.contains("/Contents <"));
    assert!(signed_str.contains("/Filter /Adobe.PPKLite"));
    // New spec-aligned SubFilter: detached PKCS#7 (not pkcs7.sha1).
    assert!(
        signed_str.contains("/SubFilter /adbe.pkcs7.detached"),
        "expected /adbe.pkcs7.detached SubFilter"
    );
}

#[test]
fn sign_and_verify_roundtrip() {
    let pdf_bytes = create_test_pdf_bytes();
    let (cert, priv_key) = generate_test_cert_and_key();

    let signer = PdfSigner::new(cert, priv_key)
        .with_reason("Approval")
        .with_location("Berlin")
        .with_contact_info("signer@example.com");

    let signed = crypto::sign_pdf(&pdf_bytes, &signer).expect("sign");
    let info = crypto::verify_pdf_signature(&signed).expect("verify");

    assert_eq!(info.reason.as_deref(), Some("Approval"));
    assert_eq!(info.location.as_deref(), Some("Berlin"));
    // New spec-aligned: is_valid is now a real RSA verification result.
    assert!(info.is_valid, "signature should be cryptographically valid");
    assert_eq!(info.signer_name.as_deref(), Some("easypdf-test"));
}

#[test]
fn verify_unsigned_pdf_fails() {
    let result = crypto::verify_pdf_signature(b"%PDF-1.4 no signature here");
    assert!(result.is_err());
    match result.unwrap_err() {
        CryptoError::InvalidSignedPdf(_) => {} // Expected.
        other => panic!("expected InvalidSignedPdf, got: {other}"),
    }
}

#[test]
fn sign_empty_content() {
    let (cert, priv_key) = generate_test_cert_and_key();
    let signer = PdfSigner::new(cert, priv_key);
    let signed = crypto::sign_pdf(b"", &signer).expect("sign empty");
    assert!(!signed.is_empty());
    assert!(String::from_utf8_lossy(&signed).contains("/Type /Sig"));
}

#[test]
fn sign_large_pdf() {
    let large_pdf = make_large_test_pdf();
    let (cert, priv_key) = generate_test_cert_and_key();
    let signer = PdfSigner::new(cert, priv_key);
    let signed = crypto::sign_pdf(&large_pdf, &signer).expect("sign large");
    assert!(signed.len() > large_pdf.len());
}

// ============================================================================
// Facade integration tests (via EasyPdf)
// ============================================================================

#[test]
fn facade_encrypt_and_decrypt_via_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("input.pdf");
    let encrypted_path = dir.path().join("encrypted.pdf");

    // Create a real PDF file.
    let pdf_bytes = create_test_pdf_bytes();
    std::fs::write(&input, &pdf_bytes).expect("write input");

    // Encrypt via facade.
    EasyPdf::encrypt(&input, &encrypted_path, "testpass").expect("encrypt");
    assert!(encrypted_path.exists());

    // Read the encrypted file and verify it IS a valid PDF with /Encrypt.
    let encrypted_bytes = std::fs::read(&encrypted_path).expect("read encrypted");
    assert!(
        encrypted_bytes.starts_with(b"%PDF"),
        "encrypted file must start with %PDF"
    );
    let s = String::from_utf8_lossy(&encrypted_bytes);
    assert!(
        s.contains("/Encrypt"),
        "encrypted file must contain /Encrypt dictionary"
    );

    // Decrypt and verify the document is no longer encrypted.
    let decrypted = crypto::decrypt_pdf(&encrypted_bytes, "testpass").expect("decrypt");
    let doc = lopdf::Document::load_mem(&decrypted).expect("decrypted must parse");
    assert!(!doc.is_encrypted());
}

#[test]
fn facade_encrypt_nonexistent_input_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let result = EasyPdf::encrypt(
        dir.path().join("nonexistent.pdf"),
        dir.path().join("out.pdf"),
        "pass",
    );
    assert!(result.is_err());
}

#[test]
fn facade_sign_nonexistent_input_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let result = EasyPdf::sign(
        dir.path().join("nonexistent.pdf"),
        dir.path().join("out.pdf"),
        dir.path().join("key.der").as_ref(),
        dir.path().join("cert.der").as_ref(),
        "reason",
    );
    assert!(result.is_err());
}

// ============================================================================
// CryptoError type tests
// ============================================================================

#[test]
fn crypto_error_display_variants() {
    let err = CryptoError::Aes("test aes".into());
    assert!(format!("{err}").contains("AES"));

    let err = CryptoError::Rsa("test rsa".into());
    assert!(format!("{err}").contains("RSA"));

    let err = CryptoError::Signature("test sig".into());
    assert!(format!("{err}").contains("signature"));

    let err = CryptoError::Verification("test ver".into());
    assert!(format!("{err}").contains("verification"));

    let err = CryptoError::InvalidEncryptedPdf("bad".into());
    assert!(format!("{err}").contains("invalid encrypted PDF"));

    let err = CryptoError::InvalidPassword("wrong".into());
    assert!(format!("{err}").contains("invalid password"));

    let err = CryptoError::InvalidSignedPdf("bad".into());
    assert!(format!("{err}").contains("invalid signed PDF"));

    let err = CryptoError::InvalidKey("bad key".into());
    assert!(format!("{err}").contains("invalid key"));

    let err = CryptoError::InvalidCertificate("bad cert".into());
    assert!(format!("{err}").contains("invalid certificate"));
}
