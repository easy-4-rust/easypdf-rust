//! # Create a Multi-Page PDF
//!
//! Demonstrates building a multi-page document with page numbers using the
//! lower-level `PdfWriter` API for full control over page-by-page content.
//!
//! Run:
//! ```sh
//! cargo run --example create_multi_page
//! ```

use easypdf::prelude::*;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = tempfile::tempdir()?;
    let out_path: PathBuf = out_dir.path().join("multi_page.pdf");

    // Use the PdfWriterBuilder for full control.
    let mut writer = EasyPdf::writer("Multi-Page Report")
        .metadata(
            PdfMetadata::new()
                .author("easypdf examples")
                .subject("Multi-page demonstration"),
        )
        .build()?;

    // Add several pages with different content.
    let chapters = [
        (
            "Chapter 1: Introduction",
            "Welcome to the multi-page example.",
        ),
        (
            "Chapter 2: Features",
            "easypdf supports tables, images, and more.",
        ),
        ("Chapter 3: Conclusion", "Thank you for trying easypdf!"),
    ];

    for (title, body) in &chapters {
        writer.add_page(PageSize::A4, Orientation::Portrait)?;

        // Write chapter title in bold.
        let title_text = PdfText::new(*title)
            .font(PdfFont::helvetica(20.0).bold())
            .color(PdfColor::blue());
        writer.write_text(&title_text, 72.0, 750.0)?;

        // Write body text.
        let body_text = PdfText::new(*body).font(PdfFont::helvetica(12.0));
        writer.write_text(&body_text, 72.0, 700.0)?;

        // Add a page number at the bottom.
        let page_num = writer.page_count();
        let page_text = PdfText::new(format!("- {page_num} -"))
            .font(PdfFont::helvetica(10.0))
            .alignment(TextAlignment::Center);
        writer.write_text(&page_text, 297.5, 30.0)?;
    }

    writer.finish(&out_path)?;
    println!("Created multi-page PDF at: {}", out_path.display());

    // Verify page count.
    let count = EasyPdf::read(&out_path).page_count()?;
    println!("Total pages: {count}");

    Ok(())
}
