//! Binary to generate special-structure PDFs for real-world testing.
//!
//! These PDFs are constructed with `lopdf` directly to exercise edge cases:
//! - encrypted (dummy `/Encrypt` dict)
//! - `AcroForm` fields
//! - image-only page (no text)
//! - deeply nested objects
//! - corrupted xref table
//!
//! Usage: `cargo run -p easypdf-test --bin generate_special_pdfs`

#![forbid(unsafe_code)]
#![allow(clippy::similar_names)]

use std::path::PathBuf;

use lopdf::{Dictionary, Document, Object, Stream};

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let samples_dir = manifest_dir.join("samples");
    std::fs::create_dir_all(&samples_dir).expect("failed to create samples dir");

    println!(
        "Generating special-structure PDFs to {}...",
        samples_dir.display()
    );

    generate_encrypted_dummy(&samples_dir.join("encrypted_dummy.pdf"));
    generate_with_acroform(&samples_dir.join("with_acroform.pdf"));
    generate_image_only(&samples_dir.join("image_only.pdf"));
    generate_nested_objects(&samples_dir.join("nested_objects.pdf"));
    generate_corrupted_xref(&samples_dir.join("corrupted_xref.pdf"));

    println!("Done. Generated:");
    for entry in std::fs::read_dir(&samples_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "pdf") {
            let size = std::fs::metadata(&path).unwrap().len();
            println!(
                "  {} ({} bytes)",
                path.file_name().unwrap().to_string_lossy(),
                size
            );
        }
    }
}

/// Helper: build a minimal page with text content and return `(doc, page_id)`.
fn make_page_with_text(doc: &mut Document, text: &str) {
    let content = format!("BT /F1 12 Tf 72 700 Td ({text}) Tj ET");
    let content_id = doc.add_object(Object::Stream(Stream::new(
        Dictionary::new(),
        content.into_bytes(),
    )));

    let mut font_dict = Dictionary::new();
    font_dict.set("Type", Object::Name(b"Font".to_vec()));
    font_dict.set("Subtype", Object::Name(b"Type1".to_vec()));
    font_dict.set("BaseFont", Object::Name(b"Helvetica".to_vec()));
    let font_id = doc.add_object(Object::Dictionary(font_dict));

    let mut resources = Dictionary::new();
    let mut fonts = Dictionary::new();
    fonts.set("F1", Object::Reference(font_id));
    resources.set("Font", Object::Dictionary(fonts));
    let resources_id = doc.add_object(Object::Dictionary(resources));

    let mut page_dict = Dictionary::new();
    page_dict.set("Type", Object::Name(b"Page".to_vec()));
    page_dict.set(
        "MediaBox",
        Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]),
    );
    page_dict.set("Contents", Object::Reference(content_id));
    page_dict.set("Resources", Object::Reference(resources_id));
    let page_id = doc.add_object(Object::Dictionary(page_dict));

    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
    pages_dict.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
    pages_dict.set("Count", Object::Integer(1));
    let pages_id = doc.add_object(Object::Dictionary(pages_dict));

    let mut catalog = Dictionary::new();
    catalog.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog.set("Pages", Object::Reference(pages_id));
    let catalog_id = doc.add_object(Object::Dictionary(catalog));

    doc.trailer.set("Root", Object::Reference(catalog_id));
}

/// Generate a PDF with a dummy `/Encrypt` dictionary in the trailer.
///
/// This does not actually encrypt the content -- it only places the
/// dictionary structure so the reader can detect the encryption marker
/// and return an appropriate error or degraded read.
fn generate_encrypted_dummy(path: &std::path::Path) {
    let mut doc = Document::with_version("1.7");
    make_page_with_text(&mut doc, "This PDF has a dummy Encrypt dictionary");

    // Build /Encrypt dictionary (Standard V2 R3 -- structure only, no real encryption)
    let mut encrypt_dict = Dictionary::new();
    encrypt_dict.set("Filter", Object::Name(b"Standard".to_vec()));
    encrypt_dict.set("V", Object::Integer(2));
    encrypt_dict.set("R", Object::Integer(3));
    encrypt_dict.set("Length", Object::Integer(128));
    // O and U are required 32-byte strings for Standard encryption
    encrypt_dict.set(
        "O",
        Object::String(vec![0u8; 32], lopdf::StringFormat::Literal),
    );
    encrypt_dict.set(
        "U",
        Object::String(vec![0u8; 32], lopdf::StringFormat::Literal),
    );
    encrypt_dict.set("P", Object::Integer(-4)); // permissions flags
    let encrypt_id = doc.add_object(Object::Dictionary(encrypt_dict));

    // Set /Encrypt reference in trailer
    doc.trailer.set("Encrypt", Object::Reference(encrypt_id));

    // Also need /ID in trailer for encrypted PDFs
    let id_array = Object::Array(vec![
        Object::String(
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
            lopdf::StringFormat::Literal,
        ),
        Object::String(
            vec![16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1],
            lopdf::StringFormat::Literal,
        ),
    ]);
    doc.trailer.set("ID", id_array);

    doc.save(path).expect("failed to save encrypted_dummy.pdf");
    println!("  generated: {}", path.display());
}

/// Generate a PDF with an `/AcroForm` dictionary in the catalog.
///
/// This simulates a PDF with interactive form fields.
fn generate_with_acroform(path: &std::path::Path) {
    let mut doc = Document::with_version("1.7");
    make_page_with_text(&mut doc, "This PDF has AcroForm fields");

    // Get the catalog object and add /AcroForm to it
    // We need to find the catalog in the document objects
    let catalog_id = doc
        .trailer
        .get(b"Root")
        .ok()
        .and_then(|r| r.as_reference().ok())
        .expect("catalog must exist");

    // Build form fields
    let mut field_dict = Dictionary::new();
    field_dict.set("Type", Object::Name(b"Annot".to_vec()));
    field_dict.set("Subtype", Object::Name(b"Widget".to_vec()));
    field_dict.set("FT", Object::Name(b"Tx".to_vec())); // Text field
    field_dict.set(
        "T",
        Object::String(b"NameField".to_vec(), lopdf::StringFormat::Literal),
    );
    field_dict.set(
        "V",
        Object::String(b"Default Value".to_vec(), lopdf::StringFormat::Literal),
    );
    field_dict.set(
        "Rect",
        Object::Array(vec![72.into(), 600.into(), 300.into(), 620.into()]),
    );
    let field_id = doc.add_object(Object::Dictionary(field_dict));

    let mut field2_dict = Dictionary::new();
    field2_dict.set("Type", Object::Name(b"Annot".to_vec()));
    field2_dict.set("Subtype", Object::Name(b"Widget".to_vec()));
    field2_dict.set("FT", Object::Name(b"Btn".to_vec())); // Button field
    field2_dict.set(
        "T",
        Object::String(b"SubmitButton".to_vec(), lopdf::StringFormat::Literal),
    );
    field2_dict.set(
        "Rect",
        Object::Array(vec![72.into(), 560.into(), 200.into(), 580.into()]),
    );
    let field2_id = doc.add_object(Object::Dictionary(field2_dict));

    // Build AcroForm dictionary
    let mut acroform_dict = Dictionary::new();
    acroform_dict.set(
        "Fields",
        Object::Array(vec![
            Object::Reference(field_id),
            Object::Reference(field2_id),
        ]),
    );
    acroform_dict.set("NeedAppearances", Object::Boolean(true));
    let acroform_id = doc.add_object(Object::Dictionary(acroform_dict));

    // Add /AcroForm to catalog
    if let Ok(Object::Dictionary(catalog)) = doc.get_object_mut(catalog_id) {
        catalog.set("AcroForm", Object::Reference(acroform_id));
    }

    doc.save(path).expect("failed to save with_acroform.pdf");
    println!("  generated: {}", path.display());
}

/// Generate a PDF with an image-only page (no text content).
///
/// This simulates a scanned page where OCR would be needed.
fn generate_image_only(path: &std::path::Path) {
    let mut doc = Document::with_version("1.7");

    // Create a minimal valid 1x1 PNG (same as used in writer tests)
    let png_data: Vec<u8> = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8,
        0xFF, 0xFF, 0x3F, 0x00, 0x05, 0xFE, 0x02, 0xFE, 0xDC, 0xCC, 0x59, 0xE7, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    // Create image XObject stream
    let mut image_dict = Dictionary::new();
    image_dict.set("Type", Object::Name(b"XObject".to_vec()));
    image_dict.set("Subtype", Object::Name(b"Image".to_vec()));
    image_dict.set("Width", Object::Integer(1));
    image_dict.set("Height", Object::Integer(1));
    image_dict.set("ColorSpace", Object::Name(b"DeviceRGB".to_vec()));
    image_dict.set("BitsPerComponent", Object::Integer(8));
    image_dict.set("Filter", Object::Name(b"FlateDecode".to_vec()));
    let image_id = doc.add_object(Object::Stream(Stream::new(image_dict, png_data)));

    // Build resources with XObject but no Font
    let mut xobject_dict = Dictionary::new();
    xobject_dict.set("Im1", Object::Reference(image_id));
    let mut resources = Dictionary::new();
    resources.set("XObject", Object::Dictionary(xobject_dict));
    let resources_id = doc.add_object(Object::Dictionary(resources));

    // Content stream that draws the image but has NO text operators (BT/ET)
    let content_bytes = b"q 595 0 0 842 0 0 cm /Im1 Do Q";
    let content_id = doc.add_object(Object::Stream(Stream::new(
        Dictionary::new(),
        content_bytes.to_vec(),
    )));

    // Page with image content only -- no /Font in resources
    let mut page_dict = Dictionary::new();
    page_dict.set("Type", Object::Name(b"Page".to_vec()));
    page_dict.set(
        "MediaBox",
        Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]),
    );
    page_dict.set("Contents", Object::Reference(content_id));
    page_dict.set("Resources", Object::Reference(resources_id));
    let page_id = doc.add_object(Object::Dictionary(page_dict));

    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
    pages_dict.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
    pages_dict.set("Count", Object::Integer(1));
    let pages_id = doc.add_object(Object::Dictionary(pages_dict));

    let mut catalog = Dictionary::new();
    catalog.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog.set("Pages", Object::Reference(pages_id));
    let catalog_id = doc.add_object(Object::Dictionary(catalog));

    doc.trailer.set("Root", Object::Reference(catalog_id));

    doc.save(path).expect("failed to save image_only.pdf");
    println!("  generated: {}", path.display());
}

/// Generate a PDF with deeply nested object references.
///
/// The nesting depth is kept within guard thresholds (default `max_element_count`
/// is 5,000,000). We create a chain of ~50 dictionaries each referencing the
/// next, which exercises the object traversal without triggering the guard.
fn generate_nested_objects(path: &std::path::Path) {
    let mut doc = Document::with_version("1.7");

    // Build a page with text first
    let content_id = doc.add_object(Object::Stream(Stream::new(
        Dictionary::new(),
        b"BT /F1 12 Tf 72 700 Td (Nested Objects Test) Tj ET".to_vec(),
    )));

    let mut font_dict = Dictionary::new();
    font_dict.set("Type", Object::Name(b"Font".to_vec()));
    font_dict.set("Subtype", Object::Name(b"Type1".to_vec()));
    font_dict.set("BaseFont", Object::Name(b"Helvetica".to_vec()));
    let font_id = doc.add_object(Object::Dictionary(font_dict));

    let mut resources = Dictionary::new();
    let mut fonts = Dictionary::new();
    fonts.set("F1", Object::Reference(font_id));
    resources.set("Font", Object::Dictionary(fonts));
    let resources_id = doc.add_object(Object::Dictionary(resources));

    // Create a chain of 50 nested dictionaries
    // Each dict has a /Next reference and a /Depth entry
    let chain_depth = 50;
    let mut prev_id = None;
    for i in (0..chain_depth).rev() {
        let mut dict = Dictionary::new();
        dict.set("Depth", Object::Integer(i));
        dict.set(
            "Label",
            Object::String(
                format!("level_{i}").into_bytes(),
                lopdf::StringFormat::Literal,
            ),
        );
        if let Some(next) = prev_id {
            dict.set("Next", Object::Reference(next));
        }
        prev_id = Some(doc.add_object(Object::Dictionary(dict)));
    }

    // The page references the head of the chain via /StructTreeRoot
    let head_id = prev_id.expect("chain must have at least one element");

    let mut page_dict = Dictionary::new();
    page_dict.set("Type", Object::Name(b"Page".to_vec()));
    page_dict.set(
        "MediaBox",
        Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]),
    );
    page_dict.set("Contents", Object::Reference(content_id));
    page_dict.set("Resources", Object::Reference(resources_id));
    let page_id = doc.add_object(Object::Dictionary(page_dict));

    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
    pages_dict.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
    pages_dict.set("Count", Object::Integer(1));
    let pages_id = doc.add_object(Object::Dictionary(pages_dict));

    let mut catalog = Dictionary::new();
    catalog.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog.set("Pages", Object::Reference(pages_id));
    catalog.set("StructTreeRoot", Object::Reference(head_id));
    let catalog_id = doc.add_object(Object::Dictionary(catalog));

    doc.trailer.set("Root", Object::Reference(catalog_id));

    doc.save(path).expect("failed to save nested_objects.pdf");
    println!("  generated: {}", path.display());
}

/// Generate a PDF then corrupt its xref table offset.
///
/// This creates a valid PDF, then overwrites the `startxref` value at the
/// end of the file with a bogus offset, forcing the reader to exercise
/// its repair path.
fn generate_corrupted_xref(path: &std::path::Path) {
    // First generate a valid PDF
    let mut doc = Document::with_version("1.7");
    make_page_with_text(&mut doc, "This PDF has a corrupted xref table");
    doc.save(path)
        .expect("failed to save base PDF for corruption");

    // Read the bytes and corrupt the startxref offset
    let mut bytes = std::fs::read(path).expect("failed to read generated PDF");

    // Find "startxref" marker near end of file
    let marker = b"startxref";
    let len = bytes.len();
    // Search backwards from the end (startxref is near EOF)
    let search_start = len.saturating_sub(200);
    let search_region = &bytes[search_start..];

    if let Some(rel_pos) = find_subsequence(search_region, marker) {
        // Position after "startxref\n"
        let abs_pos = search_start + rel_pos + marker.len();
        // Skip whitespace (newline or space)
        let mut num_start = abs_pos;
        while num_start < len
            && (bytes[num_start] == b'\n' || bytes[num_start] == b'\r' || bytes[num_start] == b' ')
        {
            num_start += 1;
        }
        // Find end of the number (before %%EOF or newline)
        let mut num_end = num_start;
        while num_end < len && bytes[num_end].is_ascii_digit() {
            num_end += 1;
        }
        // Replace the offset number with "9999999"
        let bogus = b"9999999";
        // Replace original digits with bogus value (pad/truncate to same length)
        let original_len = num_end - num_start;
        if original_len > 0 {
            for (i, byte) in bogus.iter().enumerate().take(original_len) {
                bytes[num_start + i] = *byte;
            }
        }
    }

    std::fs::write(path, &bytes).expect("failed to write corrupted PDF");
    println!("  generated: {} (xref corrupted)", path.display());
}

/// Find a byte subsequence in a slice.
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
