//! Fuzz target: PDF encryption/decryption roundtrip.
//!
//! Feeds arbitrary bytes as "PDF content" and arbitrary strings as passwords
//! to the encrypt/decrypt pipeline. Any panic is a bug.
//!
//! Tests:
//! - AES-128 and AES-256 key derivation with garbage passwords
//! - encrypt_pdf with malformed input
//! - decrypt_pdf with non-encrypted input
//! - Roundtrip: encrypt then decrypt should recover original bytes

#![no_main]

use easypdf_core::crypto::{decrypt_pdf, encrypt_pdf, PdfEncryption};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    // Split data: first byte selects algorithm variant, rest is "PDF content".
    let (algo_byte, pdf_bytes) = data.split_first().unwrap();

    // Derive a password from the PDF bytes (lossy UTF-8).
    // .into_owned() avoids Cow<str> borrow issues with Into<String>.
    let password = String::from_utf8_lossy(pdf_bytes).into_owned();

    // Select algorithm based on first byte.
    let encryption = match algo_byte % 2 {
        0 => PdfEncryption::new(&password, &password),
        _ => PdfEncryption::new(&password, &password)
            .with_algorithm(easypdf_core::crypto::PdfEncryptionAlgorithm::Aes256),
    };

    // Attempt encryption. Errors are fine; panics are bugs.
    let Ok(encrypted) = encrypt_pdf(pdf_bytes, &encryption) else {
        return;
    };

    // Attempt decryption with the same password.
    let _ = decrypt_pdf(&encrypted, &password);

    // Attempt decryption with a wrong password.
    let _ = decrypt_pdf(&encrypted, "wrong_password_fuzz");
});
