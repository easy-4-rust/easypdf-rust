//! Binary to generate golden files from sample PDFs.
//!
//! Usage: `cargo run -p easypdf-parity --bin generate_golden`
//!
//! This reads each sample PDF with easypdf and writes the extracted
//! text/metadata/structure to the `golden/` directory.

use std::path::PathBuf;

use easypdf::prelude::*;
use easypdf_test::{MetadataSnapshot, StructureSnapshot};

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let samples_dir = manifest_dir.join("samples");
    let golden_dir = manifest_dir.join("golden");

    // Ensure golden directories exist
    std::fs::create_dir_all(golden_dir.join("text-extraction")).unwrap();
    std::fs::create_dir_all(golden_dir.join("metadata")).unwrap();
    std::fs::create_dir_all(golden_dir.join("structure")).unwrap();

    let samples = ["minimal.pdf", "multipage.pdf", "with-metadata.pdf"];

    for sample in &samples {
        let pdf_path = samples_dir.join(sample);
        if !pdf_path.exists() {
            eprintln!("WARNING: sample not found: {}", pdf_path.display());
            continue;
        }

        println!("Processing {sample}...");

        // Extract text
        let text = EasyPdf::read(&pdf_path)
            .extract_text()
            .unwrap_or_else(|e| panic!("failed to extract text from {sample}: {e}"));
        let text_path = golden_dir
            .join("text-extraction")
            .join(format!("{sample}.txt"));
        std::fs::write(&text_path, &text).unwrap();
        println!("  text: {} bytes", text.len());

        // Extract metadata
        let metadata = EasyPdf::read(&pdf_path)
            .metadata()
            .unwrap_or_else(|e| panic!("failed to extract metadata from {sample}: {e}"));
        let snapshot = MetadataSnapshot::from(&metadata);
        let meta_json = serde_json::to_string_pretty(&snapshot).unwrap();
        let meta_path = golden_dir.join("metadata").join(format!("{sample}.json"));
        std::fs::write(&meta_path, &meta_json).unwrap();
        println!("  metadata: {}", meta_json.lines().next().unwrap_or(""));

        // Extract structure (page count + dimensions)
        let reader = EasyPdf::read(&pdf_path)
            .open()
            .unwrap_or_else(|e| panic!("failed to open {sample}: {e}"));
        let page_count = reader
            .page_count()
            .unwrap_or_else(|e| panic!("failed to get page count for {sample}: {e}"));

        // We can't directly get page dimensions from the current API,
        // so we record page_count only. Dimensions are verified via roundtrip.
        let structure = StructureSnapshot {
            page_count,
            pages: vec![], // Dimensions not available from current reader API
        };
        let struct_json = serde_json::to_string_pretty(&structure).unwrap();
        let struct_path = golden_dir.join("structure").join(format!("{sample}.json"));
        std::fs::write(&struct_path, &struct_json).unwrap();
        println!("  structure: {page_count} pages");
    }

    println!("\nGolden files generated. Review the diff before committing.");
}
