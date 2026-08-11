use super::*;
use super::der::{concat, der_ctx, der_int, der_seq, der_set, der_tlv};
use super::cms::build_cms_signed_data;
use super::pdf::{sign_pdf, verify_pdf_signature};
use crate::crypto::{CryptoError, PdfEncryption};

const OID_RSA_ENCRYPTION_DER: &[u8] = &[0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x01];
const OID_CN_DER: &[u8] = &[0x06, 0x03, 0x55, 0x04, 0x03];

fn der_bits(value: &[u8]) -> Vec<u8> { let mut c = Vec::with_capacity(1 + value.len()); c.push(0); c.extend(value); der_tlv(0x03, &c) }
fn der_null() -> Vec<u8> { vec![0x05, 0x00] }
fn der_utf8(s: &str) -> Vec<u8> { der_tlv(0x0C, s.as_bytes()) }
fn der_utc(s: &str) -> Vec<u8> { der_tlv(0x17, s.as_bytes()) }

fn generate_test_cert_and_key() -> (Vec<u8>, Vec<u8>) {
    use rand::rngs::OsRng;
    use rsa::pkcs1::EncodeRsaPrivateKey;
    use rsa::RsaPrivateKey;
    let mut rng = OsRng;
    let pk = RsaPrivateKey::new(&mut rng, 2048).expect("generate RSA key");
    let pub_key = pk.to_public_key();
    let priv_der = pk.to_pkcs1_der().expect("encode private key").as_bytes().to_vec();
    let cert_der = build_self_signed_cert(&priv_der, &pub_key);
    (cert_der, priv_der)
}

fn build_self_signed_cert(priv_der: &[u8], public_key: &rsa::RsaPublicKey) -> Vec<u8> {
    use ring::rand::SystemRandom;
    use ring::signature::{RsaKeyPair, RSA_PKCS1_SHA256};
    use rsa::traits::PublicKeyParts;
    let cn_attr = der_seq(&concat(&[OID_CN_DER, &der_utf8("easypdf-test")]));
    let name = der_seq(&der_set(&cn_attr));
    let validity = der_seq(&concat(&[&der_utc("200101000000Z"), &der_utc("491231235959Z")]));
    let n_bytes = public_key.n().to_bytes_be();
    let e_bytes = public_key.e().to_bytes_be();
    let rsa_pub_key = der_seq(&concat(&[&der_int(&n_bytes), &der_int(&e_bytes)]));
    let spki = der_seq(&concat(&[&der_seq(&concat(&[OID_RSA_ENCRYPTION_DER, &der_null()])), &der_bits(&rsa_pub_key)]));
    let tbs = der_seq(&concat(&[&der_ctx(0, true, &der_int(&[2u8])), &der_int(&[0x01u8]), &der_seq(OID_SHA256_WITH_RSA_DER), &name, &validity, &name, &spki]));
    let kp = RsaKeyPair::from_der(priv_der).expect("ring key parse");
    let rng = SystemRandom::new();
    let mut sig = vec![0u8; kp.public().modulus_len()];
    kp.sign(&RSA_PKCS1_SHA256, &rng, &tbs, &mut sig).expect("sign cert TBS");
    der_seq(&concat(&[&tbs, &der_seq(OID_SHA256_WITH_RSA_DER), &der_bits(&sig)]))
}

#[test]
fn signed_data_detached_builds_and_parses() {
    use x509_parser::prelude::{FromDer, X509Certificate};
    let (cert_der, _) = generate_test_cert_and_key();
    let dummy_sig = vec![0xAAu8; 256];
    let (_, cert) = X509Certificate::from_der(&cert_der).unwrap();
    let issuer_der = cert.issuer().as_raw().to_vec();
    let serial = cert.raw_serial();
    let cms = build_cms_signed_data(&cert_der, &dummy_sig, &issuer_der, serial);
    assert!(!cms.is_empty());
    let parsed = super::cms::parse_cms_signed_data(&cms).expect("parse CMS");
    assert_eq!(parsed.certificate_der, cert_der);
    assert_eq!(parsed.signature, dummy_sig);
}

#[test]
fn sign_and_verify_roundtrip_valid() {
    let (cert_der, priv_der) = generate_test_cert_and_key();
    let signer = PdfSigner::new(cert_der, priv_der).with_reason("Approval").with_location("Berlin").with_contact_info("test@example.com");
    let signed = sign_pdf(b"%PDF-1.4 test content", &signer).unwrap();
    let info = verify_pdf_signature(&signed).unwrap();
    assert!(info.is_valid);
    assert_eq!(info.reason.as_deref(), Some("Approval"));
    assert_eq!(info.location.as_deref(), Some("Berlin"));
    assert_eq!(info.signer_name.as_deref(), Some("easypdf-test"));
}

#[test]
fn verify_tampered_pdf_invalid() {
    let (cert_der, priv_der) = generate_test_cert_and_key();
    let mut signed = sign_pdf(b"%PDF-1.4 test", &PdfSigner::new(cert_der, priv_der)).unwrap();
    signed[10] = b'X';
    let info = verify_pdf_signature(&signed).unwrap();
    assert!(!info.is_valid);
}

#[test]
fn byte_range_covers_content() {
    let pdf = b"%PDF-1.4 byte range test";
    let (cert_der, priv_der) = generate_test_cert_and_key();
    let signed = sign_pdf(pdf, &PdfSigner::new(cert_der, priv_der)).unwrap();
    let br_pos = super::pdf::find_bytes(&signed, b"/ByteRange [").unwrap();
    let br_vals_start = br_pos + 12;
    let br_close = signed[br_vals_start..].iter().position(|&b| b == b']').unwrap();
    let parts: Vec<usize> = std::str::from_utf8(&signed[br_vals_start..br_vals_start + br_close]).unwrap().split_ascii_whitespace().map(|s| s.parse().unwrap()).collect();
    assert_eq!(parts.len(), 4);
    assert_eq!(parts[0], 0);
    assert_eq!(parts[1], pdf.len());
    assert_eq!(parts[2] + parts[3], signed.len());
}

#[test]
fn x509_metadata_extracted() {
    let (cert_der, priv_der) = generate_test_cert_and_key();
    let signed = sign_pdf(b"%PDF-1.4 x509", &PdfSigner::new(cert_der, priv_der).with_reason("Review").with_location("NYC")).unwrap();
    let info = verify_pdf_signature(&signed).unwrap();
    assert!(info.is_valid);
    assert_eq!(info.signer_name.as_deref(), Some("easypdf-test"));
    assert_eq!(info.reason.as_deref(), Some("Review"));
    assert_eq!(info.location.as_deref(), Some("NYC"));
}

#[test]
fn signature_dict_standard_fields() {
    let (cert_der, priv_der) = generate_test_cert_and_key();
    let signed = sign_pdf(b"%PDF-1.4 dict", &PdfSigner::new(cert_der, priv_der).with_reason("Approval").with_location("Berlin").with_contact_info("signer@example.com")).unwrap();
    let s = String::from_utf8_lossy(&signed);
    assert!(s.contains("/Type /Sig"));
    assert!(s.contains("/ByteRange"));
    assert!(s.contains("/SubFilter /adbe.pkcs7.detached"));
    assert!(s.contains("/Reason (Approval)"));
    assert!(s.contains("/Location (Berlin)"));
}

#[test]
fn verify_unsigned_pdf_fails() { assert!(verify_pdf_signature(b"%PDF-1.4 no sig").is_err()); }

#[test]
fn sign_empty_content() {
    let (c, k) = generate_test_cert_and_key();
    let signed = sign_pdf(b"", &PdfSigner::new(c, k)).unwrap();
    assert!(!signed.is_empty());
    assert!(verify_pdf_signature(&signed).unwrap().is_valid);
}

#[test]
fn sign_large_content() {
    let (c, k) = generate_test_cert_and_key();
    let signed = sign_pdf(&vec![0x42u8; 100_000], &PdfSigner::new(c, k)).unwrap();
    assert!(signed.len() > 100_000);
}

#[test]
fn sign_with_all_optional_fields() {
    let (c, k) = generate_test_cert_and_key();
    let signed = sign_pdf(b"%PDF-1.4", &PdfSigner::new(c, k).with_reason("R").with_location("L").with_contact_info("C").with_timestamp_url("http://tsa")).unwrap();
    let info = verify_pdf_signature(&signed).unwrap();
    assert!(info.is_valid);
}

#[test]
fn roundtrip_empty_reason_and_location() {
    let (c, k) = generate_test_cert_and_key();
    let info = verify_pdf_signature(&sign_pdf(b"%PDF-1.4", &PdfSigner::new(c, k)).unwrap()).unwrap();
    assert!(info.is_valid);
    assert!(info.reason.is_none());
}

#[test]
fn multiple_signatures_last_one_verifies() {
    let (c, k) = generate_test_cert_and_key();
    let signer = PdfSigner::new(c, k);
    let s1 = sign_pdf(b"%PDF-1.4 multi", &signer).unwrap();
    let s2 = sign_pdf(&s1, &signer).unwrap();
    assert!(verify_pdf_signature(&s2).unwrap().is_valid);
}

#[test]
fn crypto_error_display_variants() {
    assert!(format!("{}", CryptoError::Aes("t".into())).contains("AES"));
    assert!(format!("{}", CryptoError::Rsa("t".into())).contains("RSA"));
    assert!(format!("{}", CryptoError::Signature("t".into())).contains("signature"));
    assert!(format!("{}", CryptoError::Verification("t".into())).contains("verification"));
    assert!(format!("{}", CryptoError::InvalidEncryptedPdf("t".into())).contains("invalid encrypted PDF"));
    assert!(format!("{}", CryptoError::InvalidSignedPdf("t".into())).contains("invalid signed PDF"));
    assert!(format!("{}", CryptoError::InvalidKey("t".into())).contains("invalid key"));
    assert!(format!("{}", CryptoError::InvalidCertificate("t".into())).contains("invalid certificate"));
}

#[test]
fn pdf_encryption_new() {
    let enc = PdfEncryption::new("user", "owner");
    assert_eq!(enc.user_password, "user");
    assert_eq!(enc.owner_password, "owner");
}

#[test]
fn pdf_signer_builder_chain() {
    let s = PdfSigner::new(vec![1], vec![2]).with_reason("R").with_location("L").with_contact_info("C").with_timestamp_url("http://tsa");
    assert_eq!(s.reason.as_deref(), Some("R"));
    assert_eq!(s.location.as_deref(), Some("L"));
    assert_eq!(s.contact_info.as_deref(), Some("C"));
    assert_eq!(s.timestamp_url.as_deref(), Some("http://tsa"));
}
