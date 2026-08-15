//! # Rotate PDF Pages
//!
//! Demonstrates rotating individual pages or all pages in a PDF.
//!
//! Run:
//! ```sh
//! cargo run --example manipulate_rotate
//! ```

use easypdf::prelude::*;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = tempfile::tempdir()?;
    let source_path: PathBuf = out_dir.path().join("original.pdf");

    // Step 1: Create a 3-page source PDF.
    {
        let mut writer = EasyPdf::writer("Rotation Source").build()?;
        for i in 1..=3 {
            writer.add_page(PageSize::A4, Orientation::Portrait)?;
            let text = PdfText::new(format!("Page {i} - original orientation"))
                .font(PdfFont::helvetica(14.0));
            writer.write_text(&text, 100.0, 700.0)?;
        }
        writer.finish(&source_path)?;
    }
    println!("Created source PDF at: {}", source_path.display());

    // Step 2: Rotate page 1 by 90 degrees clockwise.
    let rotated_path = out_dir.path().join("page1_rotated.pdf");
    EasyPdf::manipulate(&source_path)
        .rotate_page(1, Rotation::Clockwise90)
        .save(&rotated_path)?;
    println!("Rotated page 1 saved at: {}", rotated_path.display());

    // Step 3: Rotate ALL pages by 180 degrees.
    let all_rotated_path = out_dir.path().join("all_rotated_180.pdf");
    EasyPdf::manipulate(&source_path)
        .rotate_all(Rotation::Clockwise180)
        .save(&all_rotated_path)?;
    println!(
        "All pages rotated 180 saved at: {}",
        all_rotated_path.display()
    );

    // Step 4: Reorder pages (reverse order).
    let reordered_path = out_dir.path().join("reversed.pdf");
    EasyPdf::manipulate(&source_path)
        .reorder_pages(&[2, 1, 0])
        .save(&reordered_path)?;
    println!("Reversed page order saved at: {}", reordered_path.display());

    Ok(())
}
