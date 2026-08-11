//! Structure parity tests (page count, page dimensions).
//!
//! Verifies that `EasyPdf::read().page_count()` matches the golden
//! structure `.json` files for each sample PDF.

use std::path::PathBuf;

use easypdf::prelude::*;
use easypdf_test::StructureSnapshot;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn samples_dir() -> PathBuf {
    manifest_dir().join("samples")
}

fn golden_struct_dir() -> PathBuf {
    manifest_dir().join("golden").join("structure")
}

fn assert_sample_structure_matches_golden(sample_name: &str) {
    let pdf_path = samples_dir().join(sample_name);
    assert!(
        pdf_path.exists(),
        "sample PDF not found: {sample_name}. Run generate_samples first."
    );

    let golden_path = golden_struct_dir().join(format!("{sample_name}.json"));
    assert!(
        golden_path.exists(),
        "golden structure not found: {}. Run generate_golden first.",
        golden_path.display()
    );

    let actual_count = EasyPdf::read(&pdf_path)
        .page_count()
        .unwrap_or_else(|e| panic!("failed to get page count for {sample_name}: {e}"));

    let golden_json = std::fs::read_to_string(&golden_path)
        .unwrap_or_else(|e| panic!("failed to read golden {}: {e}", golden_path.display()));
    let golden: StructureSnapshot = serde_json::from_str(&golden_json)
        .unwrap_or_else(|e| panic!("failed to parse golden JSON: {e}"));

    assert_eq!(
        actual_count, golden.page_count,
        "page count mismatch for {sample_name}: actual={actual_count}, golden={}",
        golden.page_count
    );
}

#[test]
fn minimal_pdf_page_count_matches_golden() {
    assert_sample_structure_matches_golden("minimal.pdf");
}

#[test]
fn multipage_pdf_page_count_matches_golden() {
    assert_sample_structure_matches_golden("multipage.pdf");
}

#[test]
fn with_metadata_pdf_page_count_matches_golden() {
    assert_sample_structure_matches_golden("with-metadata.pdf");
}
