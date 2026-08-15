//! # Split a PDF into Individual Pages
//!
//! Demonstrates splitting a multi-page PDF into separate single-page files.
//! Creates a multi-page PDF first, then splits it.
//!
//! Run:
//! ```sh
//! cargo run --example split_pdf
//! ```

use easypdf::prelude::*;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = tempfile::tempdir()?;
    let source_path: PathBuf = out_dir.path().join("multi_page.pdf");

    // Step 1: Create a multi-page PDF using PdfWriter.
    {
        let mut writer = EasyPdf::writer("Multi-Page Document").build()?;
        for i in 1..=5 {
            writer.add_page(PageSize::A4, Orientation::Portrait)?;
            let text =
                PdfText::new(format!("This is page {i} of 5.")).font(PdfFont::helvetica(16.0));
            writer.write_text(&text, 100.0, 700.0)?;
        }
        writer.finish(&source_path)?;
    }
    println!("Created multi-page PDF at: {}", source_path.display());

    // Step 2: Split into individual pages.
    let split_dir = out_dir.path().join("split_output");
    let output_paths = EasyPdf::split(&source_path)
        .every_n_pages(1)
        .save_to_dir(&split_dir)?;

    println!("Split into {} files:", output_paths.len());
    for path in &output_paths {
        println!("  {}", path.display());
    }

    // Step 3: Split into chunks of 2 pages each.
    let chunk_dir = out_dir.path().join("chunked_output");
    let chunk_paths = EasyPdf::split(&source_path)
        .every_n_pages(2)
        .save_to_dir(&chunk_dir)?;

    println!("Chunked into {} files (2 pages each):", chunk_paths.len());
    for path in &chunk_paths {
        println!("  {}", path.display());
    }

    Ok(())
}
