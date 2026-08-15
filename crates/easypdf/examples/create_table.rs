//! # Create a PDF with a Table
//!
//! Demonstrates rendering a table inside a PDF using `PdfTable` and
//! the `add_table()` builder method.
//!
//! Run:
//! ```sh
//! cargo run --example create_table
//! ```

use easypdf::prelude::*;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = tempfile::tempdir()?;
    let out_path: PathBuf = out_dir.path().join("table.pdf");

    // Build a table with headers and data rows.
    let table = PdfTable::new(vec!["Name".into(), "Role".into(), "Score".into()])
        .row(vec!["Alice".into(), "Engineer".into(), "95".into()])
        .row(vec!["Bob".into(), "Designer".into(), "88".into()])
        .row(vec!["Charlie".into(), "Manager".into(), "92".into()]);

    // Place the table in the PDF with custom column widths and row height.
    EasyPdf::create(&out_path)
        .title("Team Scores")
        .add_table(&table)
        .position(72.0, 700.0)
        .column_widths(vec![150.0, 150.0, 100.0])
        .row_height(24.0)
        .font(PdfFont::helvetica(11.0))
        .do_write()?;

    println!("Created table PDF at: {}", out_path.display());
    Ok(())
}
