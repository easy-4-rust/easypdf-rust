//! # Read a PDF
//!
//! Demonstrates reading a PDF to extract text, page count, and metadata.
//! First creates a sample PDF, then reads it back.
//!
//! Run:
//! ```sh
//! cargo run --example read_basic
//! ```

use easypdf::prelude::*;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = tempfile::tempdir()?;
    let pdf_path: PathBuf = out_dir.path().join("sample.pdf");

    // Step 1: Create a sample PDF to read.
    EasyPdf::create(&pdf_path)
        .title("Sample Document")
        .metadata(
            PdfMetadata::new()
                .author("easypdf examples")
                .subject("Read demonstration"),
        )
        .add_text("This is a sample PDF for reading.")
        .font(PdfFont::helvetica(14.0))
        .do_write()?;

    println!("Created sample PDF at: {}", pdf_path.display());

    // Step 2: Read the PDF -- extract page count.
    let page_count = EasyPdf::read(&pdf_path).page_count()?;
    println!("Page count: {page_count}");

    // Step 3: Read metadata.
    let metadata = EasyPdf::read(&pdf_path).metadata()?;
    println!("Title:  {}", metadata.title.as_deref().unwrap_or("(none)"));
    println!("Author: {}", metadata.author.as_deref().unwrap_or("(none)"));
    println!("Subject: {}", metadata.subject.as_deref().unwrap_or("(none)"));

    // Step 4: Extract text.
    let text = EasyPdf::read(&pdf_path).extract_text()?;
    println!("Extracted text ({} chars):", text.len());
    println!("---");
    println!("{text}");
    println!("---");

    Ok(())
}
