//! Text extraction parity tests.
//!
//! For each sample PDF, verifies that `EasyPdf::read().extract_text()`
//! produces output matching the golden `.txt` file.

use std::path::PathBuf;

use easypdf::prelude::*;
use easypdf_test::assert_text_eq;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn samples_dir() -> PathBuf {
    manifest_dir().join("samples")
}

fn golden_text_dir() -> PathBuf {
    manifest_dir().join("golden").join("text-extraction")
}

/// Helper: extract text from a sample and compare to golden.
fn assert_sample_text_matches_golden(sample_name: &str) {
    let pdf_path = samples_dir().join(sample_name);
    assert!(
        pdf_path.exists(),
        "sample PDF not found: {}. Run `cargo run -p easypdf-parity --bin generate_samples` first.",
        pdf_path.display()
    );

    let golden_path = golden_text_dir().join(format!("{sample_name}.txt"));
    assert!(
        golden_path.exists(),
        "golden file not found: {}. Run `cargo run -p easypdf-parity --bin generate_golden` first.",
        golden_path.display()
    );

    let actual = EasyPdf::read(&pdf_path)
        .extract_text()
        .unwrap_or_else(|e| panic!("failed to extract text from {sample_name}: {e}"));
    let golden = std::fs::read_to_string(&golden_path)
        .unwrap_or_else(|e| panic!("failed to read golden file {}: {e}", golden_path.display()));

    assert_text_eq(&actual, &golden);
}

#[test]
fn minimal_pdf_text_matches_golden() {
    assert_sample_text_matches_golden("minimal.pdf");
}

#[test]
fn multipage_pdf_text_matches_golden() {
    assert_sample_text_matches_golden("multipage.pdf");
}

#[test]
fn with_metadata_pdf_text_matches_golden() {
    assert_sample_text_matches_golden("with-metadata.pdf");
}
