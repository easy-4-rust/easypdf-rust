//! # Streaming Read Strategy
//!
//! Demonstrates reading a PDF using the `Streaming` strategy, which is
//! designed for very large documents that should not be fully loaded into memory.
//!
//! Run:
//! ```sh
//! cargo run --example streaming_read
//! ```

use easypdf::prelude::*;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = tempfile::tempdir()?;
    let pdf_path: PathBuf = out_dir.path().join("large_doc.pdf");

    // Step 1: Create a multi-page PDF to simulate a larger document.
    {
        let mut writer = EasyPdf::writer("Large Document").build()?;
        for i in 1..=10 {
            writer.add_page(PageSize::A4, Orientation::Portrait)?;
            let text = PdfText::new(format!(
                "Page {i}: This is sample content for demonstrating the streaming read strategy."
            ))
            .font(PdfFont::helvetica(12.0));
            writer.write_text(&text, 72.0, 700.0)?;
        }
        writer.finish(&pdf_path)?;
    }
    println!("Created 10-page PDF at: {}", pdf_path.display());

    // Step 2: Read with explicit Full strategy (default for small files).
    let text_full = EasyPdf::read(&pdf_path)
        .strategy(ReadStrategy::Full)
        .extract_text()?;
    println!("\n[Full] Extracted {} chars", text_full.len());

    // Step 3: Read with Lazy strategy (deferred page loading).
    let text_lazy = EasyPdf::read(&pdf_path)
        .strategy(ReadStrategy::Lazy)
        .extract_text()?;
    println!("[Lazy] Extracted {} chars", text_lazy.len());

    // Step 4: Read with Streaming strategy (incremental scan).
    // Note: Streaming has lower accuracy (no CMap resolution) but
    // uses minimal memory.
    let text_streaming = EasyPdf::read(&pdf_path)
        .strategy(ReadStrategy::Streaming)
        .extract_text()?;
    println!("[Streaming] Extracted {} chars", text_streaming.len());

    // Step 5: Read only specific pages with Lazy strategy.
    let reader = EasyPdf::read(&pdf_path)
        .strategy(ReadStrategy::Lazy)
        .pages(0..3)
        .open()?;
    let partial_text = reader.extract_text()?;
    println!("[Lazy pages 0..3] Extracted {} chars", partial_text.len());

    // Step 6: Show auto-selection logic.
    println!("\nReadStrategy auto-selection:");
    println!("  1 MB  -> {:?}", ReadStrategy::auto(1_000_000));
    println!("  50 MB -> {:?}", ReadStrategy::auto(50_000_000));
    println!("  200 MB -> {:?}", ReadStrategy::auto(200_000_000));

    Ok(())
}
