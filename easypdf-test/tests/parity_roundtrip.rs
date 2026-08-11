//! Roundtrip parity tests.
//!
//! Verifies that writing a PDF with easypdf and then reading it back
//! yields the same content as the original input. This is the fundamental
//! self-consistency check: easypdf must be able to read what it writes.

use easypdf::prelude::*;
use easypdf::PdfWriter;
use easypdf_test::{assert_text_eq, MetadataSnapshot};

/// Write a single-page PDF with given text, read it back, and verify text matches.
#[test]
fn roundtrip_single_page_text() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let pdf_path = dir.path().join("roundtrip_single.pdf");

    // Write
    EasyPdf::create(&pdf_path)
        .page_size(PageSize::A4)
        .add_text("Roundtrip test content")
        .font(PdfFont::helvetica(12.0))
        .do_write()
        .expect("failed to create PDF");

    // Read back
    let text = EasyPdf::read(&pdf_path)
        .extract_text()
        .expect("failed to extract text");

    assert_text_eq(&text, "Roundtrip test content\n");
}

/// Write a multi-page PDF, read it back, and verify all pages are present.
#[test]
fn roundtrip_multi_page_text() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let pdf_path = dir.path().join("roundtrip_multi.pdf");

    // Write 3 pages
    let mut writer = PdfWriter::new("Multi Page");
    for i in 1..=3 {
        writer
            .add_page(PageSize::A4, Orientation::Portrait)
            .expect("failed to add page");
        writer
            .write_text(
                &PdfText::new(format!("Content on page {i}")).font(PdfFont::helvetica(12.0)),
                100.0,
                700.0,
            )
            .expect("failed to write text");
    }
    writer.finish(&pdf_path).expect("failed to finish PDF");

    // Read back
    let text = EasyPdf::read(&pdf_path)
        .extract_text()
        .expect("failed to extract text");

    // Verify each page's content is present (order may vary by extractor)
    assert!(
        text.contains("Content on page 1"),
        "page 1 text missing from roundtrip output"
    );
    assert!(
        text.contains("Content on page 2"),
        "page 2 text missing from roundtrip output"
    );
    assert!(
        text.contains("Content on page 3"),
        "page 3 text missing from roundtrip output"
    );

    // Verify page count
    let count = EasyPdf::read(&pdf_path)
        .page_count()
        .expect("failed to get page count");
    assert_eq!(count, 3, "page count mismatch");
}

/// Write a PDF with metadata, read it back, and verify metadata survives.
#[test]
fn roundtrip_metadata() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let pdf_path = dir.path().join("roundtrip_meta.pdf");

    // Write with metadata
    EasyPdf::create(&pdf_path)
        .page_size(PageSize::A4)
        .metadata(
            PdfMetadata::new()
                .title("Roundtrip Title")
                .author("Parity Test"),
        )
        .add_text("Metadata roundtrip")
        .font(PdfFont::helvetica(12.0))
        .do_write()
        .expect("failed to create PDF");

    // Read back
    let meta = EasyPdf::read(&pdf_path)
        .metadata()
        .expect("failed to extract metadata");

    let snapshot = MetadataSnapshot::from(&meta);
    assert_eq!(
        snapshot.title.as_deref(),
        Some("Roundtrip Title"),
        "title lost in roundtrip"
    );
    assert_eq!(
        snapshot.author.as_deref(),
        Some("Parity Test"),
        "author lost in roundtrip"
    );
}

/// Cross-roundtrip: write -> read -> write -> read produces stable output.
///
/// This catches cases where repeated roundtrips cause drift (e.g., extra
/// whitespace, encoding changes, metadata accumulation).
#[test]
fn cross_roundtrip_stability() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let pdf1 = dir.path().join("cross_1.pdf");
    let pdf2 = dir.path().join("cross_2.pdf");

    // First write
    EasyPdf::create(&pdf1)
        .page_size(PageSize::A4)
        .add_text("Stable content")
        .font(PdfFont::helvetica(12.0))
        .do_write()
        .expect("failed to create first PDF");

    // Read from first
    let text1 = EasyPdf::read(&pdf1)
        .extract_text()
        .expect("failed to read first PDF");
    let count1 = EasyPdf::read(&pdf1)
        .page_count()
        .expect("failed to count first PDF");

    // Second write from extracted content
    let mut writer = PdfWriter::new("Cross Roundtrip");
    for _ in 0..count1 {
        writer
            .add_page(PageSize::A4, Orientation::Portrait)
            .expect("failed to add page");
    }
    writer
        .write_text(
            &PdfText::new(text1.trim()).font(PdfFont::helvetica(12.0)),
            100.0,
            700.0,
        )
        .expect("failed to write text");
    writer.finish(&pdf2).expect("failed to finish second PDF");

    // Read from second
    let text2 = EasyPdf::read(&pdf2)
        .extract_text()
        .expect("failed to read second PDF");
    let count2 = EasyPdf::read(&pdf2)
        .page_count()
        .expect("failed to count second PDF");

    // Page count must be stable
    assert_eq!(count1, count2, "page count drifted across roundtrip");

    // Text content must be stable (normalized)
    assert_text_eq(&text2, &text1);
}
