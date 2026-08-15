//! PDF signature dictionary construction, signing, and verification.
use super::PdfSigner;
use super::SignatureInfo;
use super::cms::{
    build_cms_signed_data, parse_cms_signed_data, parse_private_key, parse_x509_cert,
};
use crate::crypto::CryptoError;

const SIG_HEX_PLACEHOLDER_LEN: usize = 8192;
const BR_PREFIX: &str = "/ByteRange [";
const SIG_DICT_OPEN: &str = "\n<< /Type /Sig\n";

fn sig_dict_prefix() -> Vec<u8> {
    let mut s = String::from("\n<< /Type /Sig\n");
    s.push_str(BR_PREFIX);
    s.push_str(&" ".repeat(100));
    s.push_str("]\n/Contents <");
    s.into_bytes()
}

fn sig_dict_suffix(reason: &str, location: &str, contact: &str) -> Vec<u8> {
    use std::fmt::Write;
    let mut s = String::from(">\n");
    s.push_str("/Filter /Adobe.PPKLite\n");
    s.push_str("/SubFilter /adbe.pkcs7.detached\n");
    if !reason.is_empty() {
        let _ = writeln!(
            s,
            "/Reason ({})",
            reason.replace('(', "\\(").replace(')', "\\)")
        );
    }
    if !location.is_empty() {
        let _ = writeln!(
            s,
            "/Location ({})",
            location.replace('(', "\\(").replace(')', "\\)")
        );
    }
    if !contact.is_empty() {
        let _ = writeln!(
            s,
            "/ContactInfo ({})",
            contact.replace('(', "\\(").replace(')', "\\)")
        );
    }
    s.push_str("/M (D:20260101000000+00'00')\n>>\n");
    s.into_bytes()
}

fn trailer_section(xref_offset: usize) -> Vec<u8> {
    format!("\ntrailer\n<< /Size 1 /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n").into_bytes()
}

/// Sign a PDF document using RSA PKCS#1 v1.5 with SHA-256 and a CMS detached `SignedData` envelope.
///
/// # Errors
///
/// Returns [`CryptoError`] if signing fails.
pub fn sign_pdf(pdf_bytes: &[u8], signer: &PdfSigner) -> Result<Vec<u8>, CryptoError> {
    use ring::rand::SystemRandom;
    use x509_parser::prelude::FromDer;
    let key_pair = parse_private_key(&signer.private_key)?;
    let (_, cert) = x509_parser::prelude::X509Certificate::from_der(&signer.certificate)
        .map_err(|e| CryptoError::InvalidCertificate(format!("X.509 parse error: {e}")))?;
    let issuer_der = cert.issuer().as_raw().to_vec();
    let serial_bytes = cert.raw_serial();
    let orig_len = pdf_bytes.len();
    let reason = signer.reason.as_deref().unwrap_or("");
    let location = signer.location.as_deref().unwrap_or("");
    let contact = signer.contact_info.as_deref().unwrap_or("");
    let prefix = sig_dict_prefix();
    let suffix = sig_dict_suffix(reason, location, contact);
    let sig_val_pos = orig_len + prefix.len();
    let sig_end = sig_val_pos + SIG_HEX_PLACEHOLDER_LEN;
    let mut output =
        Vec::with_capacity(orig_len + prefix.len() + SIG_HEX_PLACEHOLDER_LEN + suffix.len() + 200);
    output.extend_from_slice(pdf_bytes);
    output.extend_from_slice(&prefix);
    output.extend(std::iter::repeat_n(b'0', SIG_HEX_PLACEHOLDER_LEN));
    output.extend_from_slice(&suffix);
    let after_sig = output.len();
    output.extend_from_slice(&trailer_section(after_sig));
    let total_len = output.len();
    let byte_range_pos = orig_len + SIG_DICT_OPEN.len() + BR_PREFIX.len();
    let byte_range_str = format!("0 {orig_len} {sig_end} {}", total_len - sig_end);
    let padded_br = format!("{byte_range_str:<100}");
    output[byte_range_pos..byte_range_pos + 100].copy_from_slice(padded_br.as_bytes());
    let mut msg = Vec::with_capacity(orig_len + (total_len - sig_end));
    msg.extend_from_slice(&output[..orig_len]);
    msg.extend_from_slice(&output[sig_end..total_len]);
    let rng = SystemRandom::new();
    let mut sig_buf = vec![0u8; key_pair.public().modulus_len()];
    key_pair
        .sign(&ring::signature::RSA_PKCS1_SHA256, &rng, &msg, &mut sig_buf)
        .map_err(|e| CryptoError::Signature(format!("{e}")))?;
    let cms = build_cms_signed_data(&signer.certificate, &sig_buf, &issuer_der, serial_bytes);
    let cms_hex = hex::encode(&cms);
    if cms_hex.len() > SIG_HEX_PLACEHOLDER_LEN {
        return Err(CryptoError::Signature(format!(
            "CMS SignedData hex ({} bytes) exceeds placeholder ({} bytes)",
            cms_hex.len(),
            SIG_HEX_PLACEHOLDER_LEN
        )));
    }
    let cms_hex_padded = format!("{cms_hex:0<SIG_HEX_PLACEHOLDER_LEN$}");
    output[sig_val_pos..sig_val_pos + SIG_HEX_PLACEHOLDER_LEN]
        .copy_from_slice(cms_hex_padded.as_bytes());
    Ok(output)
}

/// Verify a PDF digital signature.
///
/// # Errors
///
/// Returns [`CryptoError`] if verification fails.
pub fn verify_pdf_signature(pdf_bytes: &[u8]) -> Result<SignatureInfo, CryptoError> {
    let sig_start = rfind_bytes(pdf_bytes, b"/Type /Sig")
        .ok_or_else(|| CryptoError::InvalidSignedPdf("no /Type /Sig found".into()))?;
    let sig_region = &pdf_bytes[sig_start..];
    let br_pos = find_bytes(sig_region, b"/ByteRange [")
        .ok_or_else(|| CryptoError::InvalidSignedPdf("no /ByteRange found".into()))?;
    let br_vals_start = br_pos + 12;
    let br_close = sig_region[br_vals_start..]
        .iter()
        .position(|&b| b == b']')
        .ok_or_else(|| CryptoError::InvalidSignedPdf("unterminated /ByteRange".into()))?;
    let br_str = std::str::from_utf8(&sig_region[br_vals_start..br_vals_start + br_close])
        .map_err(|_| CryptoError::InvalidSignedPdf("non-UTF-8 ByteRange".into()))?
        .trim();
    let parts: Vec<usize> = br_str
        .split_whitespace()
        .map(str::parse::<usize>)
        .collect::<Result<_, _>>()
        .map_err(|_| CryptoError::InvalidSignedPdf("invalid /ByteRange values".into()))?;
    if parts.len() != 4 {
        return Err(CryptoError::InvalidSignedPdf(format!(
            "expected 4 ByteRange values, got {}",
            parts.len()
        )));
    }
    if parts[0] + parts[1] > pdf_bytes.len() || parts[2] + parts[3] > pdf_bytes.len() {
        return Err(CryptoError::InvalidSignedPdf(
            "ByteRange extends beyond PDF length".into(),
        ));
    }
    let contents_rel = find_bytes(sig_region, b"/Contents <")
        .ok_or_else(|| CryptoError::InvalidSignedPdf("no /Contents found".into()))?;
    let hex_start = sig_start + contents_rel + 11;
    let hex_close = pdf_bytes[hex_start..]
        .iter()
        .position(|&b| b == b'>')
        .ok_or_else(|| CryptoError::InvalidSignedPdf("unterminated /Contents".into()))?;
    let cms_der = hex::decode(&pdf_bytes[hex_start..hex_start + hex_close])
        .map_err(|e| CryptoError::InvalidSignedPdf(format!("invalid hex in /Contents: {e}")))?;
    let parsed_cms = parse_cms_signed_data(&cms_der)?;
    let (cert_info, public_key_der) = parse_x509_cert(&parsed_cms.certificate_der)?;
    let mut msg = Vec::with_capacity(parts[1] + parts[3]);
    msg.extend_from_slice(&pdf_bytes[parts[0]..parts[0] + parts[1]]);
    msg.extend_from_slice(&pdf_bytes[parts[2]..parts[2] + parts[3]]);
    let public_key = ring::signature::UnparsedPublicKey::new(
        &ring::signature::RSA_PKCS1_2048_8192_SHA256,
        &public_key_der,
    );
    let is_valid = public_key.verify(&msg, &parsed_cms.signature).is_ok();
    Ok(SignatureInfo {
        signer_cert: parsed_cms.certificate_der,
        signed_at: extract_paren_field_bytes(sig_region, b"/M"),
        reason: extract_paren_field_bytes(sig_region, b"/Reason"),
        location: extract_paren_field_bytes(sig_region, b"/Location"),
        is_valid,
        signer_name: cert_info.cn,
        issuer: Some(cert_info.issuer),
        cert_not_before: Some(cert_info.not_before),
        cert_not_after: Some(cert_info.not_after),
    })
}

fn rfind_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    let mut pos = None;
    for i in 0..=haystack.len() - needle.len() {
        if &haystack[i..i + needle.len()] == needle {
            pos = Some(i);
        }
    }
    pos
}

pub(super) fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    (0..=haystack.len() - needle.len()).find(|&i| &haystack[i..i + needle.len()] == needle)
}

fn extract_paren_field_bytes(text: &[u8], key: &[u8]) -> Option<String> {
    let pos = find_bytes(text, key)?;
    let after_key = &text[pos + key.len()..];
    let start = after_key.iter().position(|&b| b == b'(')? + 1;
    let end = after_key[start..].iter().position(|&b| b == b')')?;
    std::str::from_utf8(&after_key[start..start + end])
        .ok()
        .map(String::from)
}
