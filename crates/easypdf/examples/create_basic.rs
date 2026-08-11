//! # Create a Basic PDF
//!
//! Demonstrates the simplest way to create a single-page PDF with text
//! using the `EasyPdf::create()` builder chain.
//!
//! Run:
//! ```sh
//! cargo run --example create_basic
//! ```

use easypdf::prelude::*;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = tempfile::tempdir()?;
    let out_path: PathBuf = out_dir.path().join("hello.pdf");

    // Create a PDF with a title, custom font, and colored text.
    let path = EasyPdf::create(&out_path)
        .title("Hello World")
        .add_text("Hello, easypdf!")
        .font(PdfFont::helvetica(18.0))
        .do_write()?;

    println!("Created PDF at: {}", path.display());

    // Also show how to use positioned text with color.
    let out_path2 = out_dir.path().join("styled.pdf");
    EasyPdf::create(&out_path2)
        .title("Styled Text")
        .add_text("This is bold and red.")
        .font(PdfFont::helvetica(14.0).bold())
        .position(72.0, 700.0)
        .do_write()?;

    println!("Created styled PDF at: {}", out_path2.display());
    Ok(())
}
