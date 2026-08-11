//! Fuzz target: PDF signing with arbitrary keys and content.
//!
//! Tests the sign_pdf code path with garbage certificates and private keys.
//! The signing operation should return an error (invalid key) but must
//! never panic.
//!
//! Note: verify_pdf_signature is not tested here because it requires a
//! structurally valid signed PDF, which sign_pdf won't produce with garbage keys.

#![no_main]

use easypdf_core::crypto::{sign_pdf, verify_pdf_signature, PdfSigner};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }

    // Split data into "certificate", "private key", and "PDF content".
    let third = data.len() / 3;
    let (cert_and_rest, pdf_bytes) = data.split_at(third.min(data.len()));
    let (cert_bytes, key_bytes) = cert_and_rest.split_at(third.min(cert_and_rest.len()));

    let signer = PdfSigner::new(cert_bytes.to_vec(), key_bytes.to_vec())
        .with_reason("fuzz-test");

    // Attempt signing. With garbage keys this should fail, but must not panic.
    let Ok(signed) = sign_pdf(pdf_bytes, &signer) else {
        return;
    };

    // If signing somehow succeeded, attempt verification on the result.
    // This should either succeed or fail gracefully.
    let _ = verify_pdf_signature(&signed);

    // Also try verifying the original (unsigned) bytes -- must not panic.
    let _ = verify_pdf_signature(pdf_bytes);
});
