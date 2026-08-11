//! # Fill a PDF Form
//!
//! Demonstrates the PDF form-filling API using `PdfFillBuilder` and
//! the `#[derive(PdfModel)]` macro for type-safe field mapping.
//!
//! **Note**: This example shows the API patterns. Actual form filling
//! requires a PDF template with interactive form fields (AcroForm).
//!
//! Run:
//! ```sh
//! cargo run --example fill_form
//! ```

use easypdf::prelude::*;
use std::path::PathBuf;

/// Example data model for an invoice, using the `PdfModel` derive macro.
///
/// The `#[pdf(field = "...")]` attribute maps struct fields to PDF form
/// field names in the template.
#[derive(PdfModel)]
struct Invoice {
    /// Maps to the "customer_name" field in the PDF template.
    #[pdf(field = "customer_name")]
    customer: String,

    /// Maps to the "invoice_number" field.
    #[pdf(field = "invoice_number")]
    invoice_number: String,

    /// Maps to the "total_amount" field.
    #[pdf(field = "amount")]
    total: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = tempfile::tempdir()?;

    // --- Demonstrate the derive macro ---
    let invoice = Invoice {
        customer: "Acme Corp".to_string(),
        invoice_number: "INV-2025-001".to_string(),
        total: 1234.56,
    };

    // PdfModel trait provides field descriptors for introspection.
    let descriptors = invoice.field_descriptors();
    println!("Invoice has {} form field(s):", descriptors.len());
    for desc in &descriptors {
        println!(
            "  field={:?} -> rust_field={:?} (order={}, required={})",
            desc.field_name, desc.rust_field_name, desc.order, desc.required
        );
    }

    // --- Demonstrate the fill builder API ---
    // In a real scenario, you would have a PDF template with AcroForm fields.
    // Here we show the builder chain pattern:
    //
    //   EasyPdf::fill_form("template.pdf", &invoice)
    //       .field("extra_note", "Please pay within 30 days")
    //       .save("filled.pdf")?;

    // Since we don't have a real template, demonstrate the manual field API
    // by creating a simple output PDF with the field values rendered as text.
    let out_path: PathBuf = out_dir.path().join("invoice_output.pdf");

    EasyPdf::create(&out_path)
        .title("Invoice")
        .add_text(format!("Customer: {}", invoice.customer))
        .font(PdfFont::helvetica(14.0))
        .position(72.0, 750.0)
        .do_write()?;

    // Append more fields on a second pass (for demonstration).
    // In practice, use PdfWriter for multi-line content.
    let mut writer = EasyPdf::writer("Invoice Details")
        .build()?;
    writer.add_page(PageSize::A4, Orientation::Portrait)?;

    let lines = [
        format!("Customer: {}", invoice.customer),
        format!("Invoice #: {}", invoice.invoice_number),
        format!("Total: ${:.2}", invoice.total),
    ];
    for (i, line) in lines.iter().enumerate() {
        let y = 750.0 - f64::from(i as u32) * 30.0;
        let text = PdfText::new(line).font(PdfFont::helvetica(14.0));
        writer.write_text(&text, 72.0, y)?;
    }

    let detail_path = out_dir.path().join("invoice_detailed.pdf");
    writer.finish(&detail_path)?;
    println!("\nCreated invoice PDF at: {}", detail_path.display());

    Ok(())
}
