//! # Convert PDF to Markdown
//!
//! Demonstrates converting a PDF to Markdown text using the in-memory
//! `to_markdown()` API. Requires the `markdown` feature.
//!
//! Run:
//! ```sh
//! cargo run --example pdf_to_markdown --features markdown
//! ```

use easypdf::prelude::*;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = tempfile::tempdir()?;
    let pdf_path: PathBuf = out_dir.path().join("source.pdf");

    // Step 1: Create a source PDF with some text content.
    {
        let mut writer = EasyPdf::writer("Markdown Source").build()?;
        writer.add_page(PageSize::A4, Orientation::Portrait)?;

        let title = PdfText::new("Annual Report 2025")
            .font(PdfFont::helvetica(20.0).bold());
        writer.write_text(&title, 72.0, 750.0)?;

        let body = PdfText::new(
            "This document demonstrates PDF to Markdown conversion. \
             The easypdf library can extract structured text from PDFs \
             and render it as GitHub Flavored Markdown.",
        )
        .font(PdfFont::helvetica(12.0));
        writer.write_text(&body, 72.0, 700.0)?;

        writer.finish(&pdf_path)?;
    }
    println!("Created source PDF at: {}", pdf_path.display());

    // Step 2: Convert to Markdown (in-memory).
    let result = EasyPdf::to_markdown(&pdf_path).do_convert()?;

    println!("\n--- Markdown Output ---");
    println!("{}", result.markdown());
    println!("--- End ---");

    // Print conversion report.
    let report = result.report();
    println!("\nConversion report:");
    println!("  Pages read:     {}", report.pages_read());
    println!("  Blocks written: {}", report.blocks_written());
    println!("  Bytes written:  {}", report.bytes_written());
    if !report.warnings().is_empty() {
        println!("  Warnings:       {}", report.warnings().len());
    }

    // Step 3: Export to a .md file using the file-based API.
    let md_path = out_dir.path().join("output.md");
    let export = EasyPdf::export_markdown(&pdf_path, &md_path)
        .profile(MarkdownProfile::Gfm)
        .do_export()?;

    println!("\nExported Markdown to: {}", export.output().display());
    println!("  Export pages: {}", export.report().pages_read());

    Ok(())
}
