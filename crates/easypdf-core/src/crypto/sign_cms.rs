//! CMS `SignedData` construction and parsing (RFC 5652).
use crate::crypto::CryptoError;
use super::der::{concat, der_ctx, der_int, der_octets, der_seq, der_set, DerReader};
use super::{OID_DATA_DER, OID_SHA256_DER, OID_SHA256_WITH_RSA_DER, OID_SIGNED_DATA_DER, OID_SIGNED_DATA_VAL};

pub(super) fn build_cms_signed_data(cert_der: &[u8], signature: &[u8], issuer_der: &[u8], serial_number: &[u8]) -> Vec<u8> {
    let digest_alg = der_seq(OID_SHA256_DER);
    let sig_alg = der_seq(OID_SHA256_WITH_RSA_DER);
    let encap_content = der_seq(OID_DATA_DER);
    let issuer_and_serial = der_seq(&concat(&[issuer_der, &der_int(serial_number)]));
    let signer_info = der_seq(&concat(&[&der_int(&[1u8]), &issuer_and_serial, &digest_alg, &sig_alg, &der_octets(signature)]));
    let signer_infos = der_set(&signer_info);
    let certs = der_ctx(0, true, cert_der);
    let digest_algs = der_set(&digest_alg);
    let signed_data = der_seq(&concat(&[&der_int(&[1u8]), &digest_algs, &encap_content, &certs, &signer_infos]));
    der_seq(&concat(&[OID_SIGNED_DATA_DER, &der_ctx(0, true, &signed_data)]))
}

pub(super) struct ParsedCms { pub(super) certificate_der: Vec<u8>, pub(super) signature: Vec<u8> }

pub(super) fn parse_cms_signed_data(der: &[u8]) -> Result<ParsedCms, CryptoError> {
    let mut reader = DerReader::new(der);
    let ci_bytes = reader.read_sequence()?;
    let mut ci = DerReader::new(ci_bytes);
    let content_type = ci.read_oid()?;
    if content_type != OID_SIGNED_DATA_VAL { return Err(CryptoError::InvalidSignedPdf("CMS content type is not signedData".into())); }
    let (tag, inner) = ci.read_tlv().map_err(|_| CryptoError::InvalidSignedPdf("missing CMS content".into()))?;
    if tag != 0xA0 { return Err(CryptoError::InvalidSignedPdf("expected [0] EXPLICIT tag for SignedData".into())); }
    let mut sd = DerReader::new(inner);
    let sd_bytes = sd.read_sequence()?;
    let mut s = DerReader::new(sd_bytes);
    let version = s.read_integer()?;
    if version != [1u8] { return Err(CryptoError::InvalidSignedPdf(format!("unsupported SignedData version: {version:?}"))); }
    let _ = s.read_set()?;
    let _ = s.read_sequence()?;
    let certs_der = s.read_ctx_implicit(0)?.ok_or_else(|| CryptoError::InvalidSignedPdf("CMS missing certificates".into()))?;
    let mut cert_reader = DerReader::new(certs_der);
    let (cert_tag, cert_value) = cert_reader.read_tlv()?;
    if cert_tag != 0x30 { return Err(CryptoError::InvalidSignedPdf("expected SEQUENCE for certificate in CMS".into())); }
    let first_cert = der_seq(cert_value);
    let signer_infos_bytes = s.read_set()?;
    let mut infos_iter = DerReader::new(signer_infos_bytes);
    let first_info = infos_iter.read_sequence()?;
    let mut si = DerReader::new(first_info);
    let si_version = si.read_integer()?;
    if si_version != [1u8] { return Err(CryptoError::InvalidSignedPdf(format!("unsupported SignerInfo version: {si_version:?}"))); }
    let _sid = si.read_sequence()?;
    let _ = si.read_sequence()?;
    if si.peek_tag() == Some(0xA0) { si.skip_field()?; }
    let _ = si.read_sequence()?;
    let signature = si.read_octet_string()?.to_vec();
    Ok(ParsedCms { certificate_der: first_cert, signature })
}

#[derive(Debug, Clone)]
pub(super) struct CertificateInfo { pub(super) cn: Option<String>, pub(super) issuer: String, pub(super) not_before: String, pub(super) not_after: String }

pub(super) fn parse_x509_cert(cert_der: &[u8]) -> Result<(CertificateInfo, Vec<u8>), CryptoError> {
    use x509_parser::prelude::*;
    let (_, cert) = X509Certificate::from_der(cert_der).map_err(|e| CryptoError::InvalidCertificate(format!("X.509 parse error: {e}")))?;
    let cn = cert.subject().iter_common_name().next().and_then(|attr| attr.as_str().ok()).map(String::from);
    let issuer = format!("{}", cert.issuer());
    let validity = cert.validity();
    let info = CertificateInfo { cn, issuer, not_before: format!("{}", validity.not_before), not_after: format!("{}", validity.not_after) };
    let spki = cert.public_key();
    let public_key_der = spki.subject_public_key.as_ref().to_vec();
    Ok((info, public_key_der))
}

pub(super) fn parse_private_key(der: &[u8]) -> Result<ring::signature::RsaKeyPair, CryptoError> {
    if let Ok(key) = ring::signature::RsaKeyPair::from_pkcs8(der) { return Ok(key); }
    ring::signature::RsaKeyPair::from_der(der).map_err(|e| CryptoError::InvalidKey(format!("RSA key parse error: {e}")))
}
