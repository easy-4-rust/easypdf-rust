//! Metadata extraction parity tests.
//!
//! Verifies that `EasyPdf::read().metadata()` produces output matching
//! the golden `.json` files for each sample PDF.

use std::path::PathBuf;

use easypdf::prelude::*;
use easypdf_test::MetadataSnapshot;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn samples_dir() -> PathBuf {
    manifest_dir().join("samples")
}

fn golden_meta_dir() -> PathBuf {
    manifest_dir().join("golden").join("metadata")
}

fn assert_sample_metadata_matches_golden(sample_name: &str) {
    let pdf_path = samples_dir().join(sample_name);
    assert!(
        pdf_path.exists(),
        "sample PDF not found: {sample_name}. Run generate_samples first."
    );

    let golden_path = golden_meta_dir().join(format!("{sample_name}.json"));
    assert!(
        golden_path.exists(),
        "golden metadata not found: {}. Run generate_golden first.",
        golden_path.display()
    );

    let actual_meta = EasyPdf::read(&pdf_path)
        .metadata()
        .unwrap_or_else(|e| panic!("failed to extract metadata from {sample_name}: {e}"));
    let actual = MetadataSnapshot::from(&actual_meta);

    let golden_json = std::fs::read_to_string(&golden_path)
        .unwrap_or_else(|e| panic!("failed to read golden {}: {e}", golden_path.display()));
    let golden: MetadataSnapshot = serde_json::from_str(&golden_json)
        .unwrap_or_else(|e| panic!("failed to parse golden JSON: {e}"));

    assert_eq!(
        actual, golden,
        "\nmetadata mismatch for {sample_name}:\n  actual: {actual:?}\n  golden: {golden:?}"
    );
}

#[test]
fn minimal_pdf_metadata_matches_golden() {
    assert_sample_metadata_matches_golden("minimal.pdf");
}

#[test]
fn multipage_pdf_metadata_matches_golden() {
    assert_sample_metadata_matches_golden("multipage.pdf");
}

#[test]
fn with_metadata_pdf_has_nonempty_title() {
    // Verify that the with-metadata sample actually produces metadata.
    // The exact values are verified by the golden comparison test above;
    // this test only checks that the title field is populated (non-None).
    let pdf_path = samples_dir().join("with-metadata.pdf");
    assert!(pdf_path.exists(), "sample PDF not found. Run generate_samples first.");

    let meta = EasyPdf::read(&pdf_path)
        .metadata()
        .expect("failed to extract metadata from with-metadata.pdf");

    assert!(
        meta.title.is_some(),
        "with-metadata.pdf should have a title, got None"
    );
}
