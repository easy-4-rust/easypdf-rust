//! Unit tests for encrypt module.

use super::*;
use crate::crypto::CryptoError;

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
    let enc =
        PdfEncryption::new("user123", "owner456").with_algorithm(PdfEncryptionAlgorithm::Aes128);

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
    let enc =
        PdfEncryption::new("user123", "owner456").with_algorithm(PdfEncryptionAlgorithm::Aes256);

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
    let enc = PdfEncryption::new("u", "o").with_algorithm(PdfEncryptionAlgorithm::Aes128);
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
    let enc = PdfEncryption::new("u", "o").with_algorithm(PdfEncryptionAlgorithm::Aes256);
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

    let enc128 = PdfEncryption::new("u", "o").with_algorithm(PdfEncryptionAlgorithm::Aes128);
    let encrypted = encrypt_pdf(&pdf, &enc128).unwrap();
    let info = get_encryption_info(&encrypted).unwrap().unwrap();
    assert_eq!(info.algorithm, PdfEncryptionAlgorithm::Aes128);
    assert_eq!(info.version, 4);
    assert_eq!(info.revision, 4);

    let enc256 = PdfEncryption::new("u", "o").with_algorithm(PdfEncryptionAlgorithm::Aes256);
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
    let enc = PdfEncryption::new("u", "o").with_algorithm(PdfEncryptionAlgorithm::Aes128);
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
