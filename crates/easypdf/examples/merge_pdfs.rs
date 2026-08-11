//! # Merge Multiple PDFs
//!
//! Demonstrates merging several single-page PDFs into one combined document.
//!
//! Run:
//! ```sh
//! cargo run --example merge_pdfs
//! ```

use easypdf::prelude::*;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = tempfile::tempdir()?;

    // Step 1: Create three single-page PDFs.
    let mut source_paths = Vec::new();
    for i in 1..=3 {
        let path: PathBuf = out_dir.path().join(format!("page_{i}.pdf"));
        EasyPdf::create(&path)
            .title(format!("Part {i}"))
            .add_text(format!("This is page {i} of the merged document."))
            .font(PdfFont::helvetica(14.0))
            .do_write()?;
        println!("Created source PDF: {}", path.display());
        source_paths.push(path);
    }

    // Step 2: Merge them into a single output PDF.
    let merged_path = out_dir.path().join("merged.pdf");
    EasyPdf::merge(&source_paths, &merged_path)?;

    println!("Merged PDF created at: {}", merged_path.display());

    // Step 3: Verify the merged result.
    let page_count = EasyPdf::read(&merged_path).page_count()?;
    println!("Merged PDF page count: {page_count}");

    Ok(())
}
