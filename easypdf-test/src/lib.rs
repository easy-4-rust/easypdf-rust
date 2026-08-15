//! easypdf-rust acceptance test crate.
//!
//! Cross-crate public API workflows and behavior parity verification.

#![forbid(unsafe_code)]

use std::path::Path;

use easypdf::PdfWriter;
use easypdf::prelude::*;
use serde::{Deserialize, Serialize};

/// Normalized comparison for text extraction.
///
/// Normalizes line endings and trims whitespace to avoid platform-specific
/// differences while still catching content changes.
///
/// # Panics
///
/// Panics if the normalized texts do not match.
pub fn assert_text_eq(actual: &str, golden: &str) {
    let normalize = |s: &str| {
        s.replace("\r\n", "\n")
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    };
    let actual_norm = normalize(actual);
    let golden_norm = normalize(golden);
    assert_eq!(
        actual_norm, golden_norm,
        "\n--- actual ---\n{actual_norm}\n--- golden ---\n{golden_norm}\n"
    );
}

/// Metadata snapshot for golden comparison.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetadataSnapshot {
    /// Document title.
    pub title: Option<String>,
    /// Document author.
    pub author: Option<String>,
    /// Document subject.
    pub subject: Option<String>,
    /// Keywords.
    pub keywords: Option<String>,
    /// Creator application.
    pub creator: Option<String>,
    /// Producer application.
    pub producer: Option<String>,
}

impl From<&PdfMetadata> for MetadataSnapshot {
    fn from(m: &PdfMetadata) -> Self {
        Self {
            title: m.title.clone(),
            author: m.author.clone(),
            subject: m.subject.clone(),
            keywords: m.keywords.clone(),
            creator: m.creator.clone(),
            producer: m.producer.clone(),
        }
    }
}

/// Structure snapshot for golden comparison.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructureSnapshot {
    /// Number of pages.
    pub page_count: usize,
    /// Page dimensions (width, height) in PDF points for each page.
    pub pages: Vec<PageDimensions>,
}

/// Dimensions of a single page.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PageDimensions {
    /// Page width in PDF points.
    pub width: f64,
    /// Page height in PDF points.
    pub height: f64,
}

/// Create the `minimal.pdf` sample: one page with "Hello World".
///
/// # Errors
///
/// Returns an error if PDF creation fails.
pub fn create_minimal_pdf(path: &Path) -> easypdf::Result<()> {
    EasyPdf::create(path)
        .page_size(PageSize::A4)
        .add_text("Hello World")
        .font(PdfFont::helvetica(12.0))
        .do_write()?;
    Ok(())
}

/// Create the `multipage.pdf` sample: three pages.
///
/// # Errors
///
/// Returns an error if PDF creation fails.
pub fn create_multipage_pdf(path: &Path) -> easypdf::Result<()> {
    let mut writer = PdfWriter::new("Multipage Test");
    for i in 1..=3 {
        writer.add_page(PageSize::A4, Orientation::Portrait)?;
        writer.write_text(
            &PdfText::new(format!("Page {i}")).font(PdfFont::helvetica(14.0)),
            100.0,
            700.0,
        )?;
    }
    writer.finish(path)?;
    Ok(())
}

/// Create the `with-metadata.pdf` sample: one page with Title and Author.
///
/// # Errors
///
/// Returns an error if PDF creation fails.
pub fn create_with_metadata_pdf(path: &Path) -> easypdf::Result<()> {
    EasyPdf::create(path)
        .page_size(PageSize::A4)
        .metadata(
            PdfMetadata::new()
                .title("Test Document")
                .author("easypdf-rust"),
        )
        .add_text("Document with metadata")
        .font(PdfFont::helvetica(12.0))
        .do_write()?;
    Ok(())
}

/// Create the `large_100page.pdf` sample: 100 pages for stress/performance testing.
///
/// # Errors
///
/// Returns an error if PDF creation fails.
pub fn create_large_100page_pdf(path: &Path) -> easypdf::Result<()> {
    let mut writer = PdfWriter::new("Large Document Test");
    for i in 1..=100 {
        writer.add_page(PageSize::A4, Orientation::Portrait)?;
        writer.write_text(
            &PdfText::new(format!(
                "Page {i} of 100 — Stress test document for performance benchmarking."
            ))
            .font(PdfFont::helvetica(12.0)),
            72.0,
            780.0,
        )?;
        writer.write_text(
            &PdfText::new(format!("Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Page {i}."))
                .font(PdfFont::helvetica(10.0)),
            72.0,
            750.0,
        )?;
    }
    writer.finish(path)?;
    Ok(())
}

/// Create the `with_table_text.pdf` sample: pipe-separated table text for table detection.
///
/// # Errors
///
/// Returns an error if PDF creation fails.
pub fn create_with_table_text_pdf(path: &Path) -> easypdf::Result<()> {
    let mut writer = PdfWriter::new("Table Detection Test");
    writer.add_page(PageSize::A4, Orientation::Portrait)?;

    let table_lines = [
        "|Name|Age|City|",
        "|---|---|---|",
        "|Alice|30|NYC|",
        "|Bob|25|LA|",
        "|Charlie|35|Chicago|",
        "|Diana|28|Houston|",
        "|Eve|32|Phoenix|",
    ];

    let mut y = 780.0;
    for line in &table_lines {
        writer.write_text(&PdfText::new(*line).font(PdfFont::courier(10.0)), 72.0, y)?;
        y -= 16.0;
    }

    // Additional non-table text below
    writer.write_text(
        &PdfText::new("The above is a table with pipe-delimited columns for detection testing.")
            .font(PdfFont::helvetica(11.0)),
        72.0,
        y - 20.0,
    )?;

    writer.finish(path)?;
    Ok(())
}

/// Create the `multi_column_heuristic.pdf` sample: multi-paragraph text for reading order.
///
/// # Errors
///
/// Returns an error if PDF creation fails.
pub fn create_multi_column_heuristic_pdf(path: &Path) -> easypdf::Result<()> {
    let mut writer = PdfWriter::new("Multi-Column Heuristic Test");
    writer.add_page(PageSize::A4, Orientation::Portrait)?;

    // Left column area (short paragraphs)
    let left_paragraphs = [
        "Introduction",
        "This is a short introductory paragraph placed in the left portion of the page. It provides context for the document layout.",
        "Methods",
        "We used synthetic PDF generation to create test fixtures with known content for regression testing.",
    ];

    // Right column area (different length paragraphs)
    let right_paragraphs = [
        "Results",
        "The extraction pipeline successfully identified paragraph boundaries across multiple layout configurations. Longer paragraphs with varying sentence structures were correctly segmented.",
        "Conclusion",
        "Synthetic PDFs provide reliable test coverage.",
    ];

    // Write left column
    let mut y = 780.0;
    for text in &left_paragraphs {
        writer.write_text(&PdfText::new(*text).font(PdfFont::helvetica(11.0)), 72.0, y)?;
        y -= 18.0;
    }

    // Write right column
    y = 780.0;
    for text in &right_paragraphs {
        writer.write_text(
            &PdfText::new(*text).font(PdfFont::helvetica(11.0)),
            320.0,
            y,
        )?;
        y -= 18.0;
    }

    writer.finish(path)?;
    Ok(())
}

/// Generate all sample PDFs to the given directory.
///
/// # Errors
///
/// Returns an error if any PDF cannot be created.
pub fn generate_all_samples(samples_dir: &Path) -> easypdf::Result<()> {
    std::fs::create_dir_all(samples_dir)?;
    create_minimal_pdf(&samples_dir.join("minimal.pdf"))?;
    create_multipage_pdf(&samples_dir.join("multipage.pdf"))?;
    create_with_metadata_pdf(&samples_dir.join("with-metadata.pdf"))?;
    create_large_100page_pdf(&samples_dir.join("large_100page.pdf"))?;
    create_with_table_text_pdf(&samples_dir.join("with_table_text.pdf"))?;
    create_multi_column_heuristic_pdf(&samples_dir.join("multi_column_heuristic.pdf"))?;
    Ok(())
}
