//! # Markdown Conversion Pipeline
//!
//! Demonstrates creating a custom Markdown conversion pipeline with
//! profile selection and strategy configuration. Requires the `markdown` feature.
//!
//! Run:
//! ```sh
//! cargo run --example markdown_pipeline --features markdown
//! ```

use easypdf::prelude::*;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = tempfile::tempdir()?;
    let pdf_path: PathBuf = out_dir.path().join("pipeline_source.pdf");

    // Step 1: Create a source PDF.
    {
        let mut writer = EasyPdf::writer("Pipeline Source").build()?;
        writer.add_page(PageSize::A4, Orientation::Portrait)?;

        let heading = PdfText::new("Pipeline Demo")
            .font(PdfFont::helvetica(18.0).bold());
        writer.write_text(&heading, 72.0, 750.0)?;

        let body = PdfText::new(
            "This example shows how to configure the Markdown conversion \
             pipeline with different profiles and strategies.",
        )
        .font(PdfFont::helvetica(12.0));
        writer.write_text(&body, 72.0, 700.0)?;

        writer.finish(&pdf_path)?;
    }
    println!("Created source PDF: {}", pdf_path.display());

    // Step 2: Create a pipeline with the GFM profile.
    let _pipeline = EasyPdf::markdown_pipeline(MarkdownProfile::Gfm);
    println!("Created GFM pipeline (empty, ready for processors).");

    // Step 3: Convert with different profiles.
    for profile in [MarkdownProfile::Gfm, MarkdownProfile::Llm, MarkdownProfile::Plain] {
        let result = EasyPdf::to_markdown(&pdf_path)
            .profile(profile)
            .do_convert()?;

        println!(
            "\nProfile {:?}: {} bytes, {} blocks",
            profile,
            result.report().bytes_written(),
            result.report().blocks_written(),
        );
    }

    // Step 4: Convert with different table and OCR strategies.
    let result = EasyPdf::to_markdown(&pdf_path)
        .profile(MarkdownProfile::Gfm)
        .tables(TablePolicy::Detect)
        .ocr(OcrPolicy::Disabled)
        .do_convert()?;

    println!(
        "\nWith table detection + no OCR: {} bytes",
        result.report().bytes_written()
    );

    Ok(())
}
