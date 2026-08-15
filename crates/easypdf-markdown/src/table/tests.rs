//! Integration-level tests for the table detection processor.

use crate::{PdfMarkdownProcessor, ProcessorPipeline};
use easypdf_core::PdfInput;
use easypdf_core::{PageIndex, PdfMetadata};
use easypdf_core::{PdfBlock, PdfBlockType, PdfDocumentModel, PdfPageModel, SourceLocation};

use super::config::{ColumnSeparator, TableDetectionConfig};
use super::detector::TableDetectorProcessor;

fn loc() -> SourceLocation {
    SourceLocation::new(PageIndex::new(0), 1.0)
}

fn para(text: &str) -> PdfBlock {
    PdfBlock::paragraph(text, loc())
}

fn make_doc(blocks: Vec<PdfBlock>) -> PdfDocumentModel {
    let mut page = PdfPageModel::new(PageIndex::new(0));
    for block in blocks {
        page = page.with_block(block);
    }
    PdfDocumentModel::new(PdfMetadata::default(), vec![page])
}

fn empty_input() -> PdfInput {
    PdfInput::from_bytes(Vec::new())
}

fn run_process(doc: PdfDocumentModel) -> PdfDocumentModel {
    let proc = TableDetectorProcessor::new();
    let (result, warnings) = proc.process(&empty_input(), doc).unwrap();
    assert!(warnings.is_empty());
    result
}

fn count_tables(doc: &PdfDocumentModel) -> usize {
    doc.iter_all_blocks()
        .filter(|(_, b)| b.block_type() == PdfBlockType::Table)
        .count()
}

// =========================================================================
// Pipe-separated tables
// =========================================================================

#[test]
fn pipe_separated_basic() {
    let doc = make_doc(vec![
        para("| Name | Age |"),
        para("| --- | --- |"),
        para("| Alice | 30 |"),
        para("| Bob | 25 |"),
    ]);
    let result = run_process(doc);
    assert_eq!(count_tables(&result), 1);
    let blocks: Vec<_> = result.iter_all_blocks().collect();
    assert_eq!(blocks.len(), 1);
    if let PdfBlock::Table { headers, rows, .. } = blocks[0].1 {
        assert_eq!(headers, &["Name", "Age"]);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec!["Alice", "30"]);
        assert_eq!(rows[1], vec!["Bob", "25"]);
    } else {
        panic!("expected Table block");
    }
}

#[test]
fn pipe_separated_no_separator_line() {
    let doc = make_doc(vec![
        para("| Name | Age |"),
        para("| Alice | 30 |"),
        para("| Bob | 25 |"),
    ]);
    let result = run_process(doc);
    assert_eq!(count_tables(&result), 1);
}

#[test]
fn pipe_separated_with_surrounding_text() {
    let doc = make_doc(vec![
        para("Introduction paragraph."),
        para("| Col1 | Col2 |"),
        para("| --- | --- |"),
        para("| A | B |"),
        para("Closing paragraph."),
    ]);
    let result = run_process(doc);
    let blocks: Vec<_> = result.iter_all_blocks().collect();
    assert_eq!(blocks.len(), 3);
    assert!(matches!(blocks[0].1, PdfBlock::Paragraph { .. }));
    assert!(matches!(blocks[1].1, PdfBlock::Table { .. }));
    assert!(matches!(blocks[2].1, PdfBlock::Paragraph { .. }));
}

// =========================================================================
// Tab-separated tables
// =========================================================================

#[test]
fn tab_separated_basic() {
    let doc = make_doc(vec![
        para("Name\tAge\tCity"),
        para("Alice\t30\tNYC"),
        para("Bob\t25\tLA"),
    ]);
    let result = run_process(doc);
    assert_eq!(count_tables(&result), 1);
    let blocks: Vec<_> = result.iter_all_blocks().collect();
    if let PdfBlock::Table { headers, rows, .. } = blocks[0].1 {
        assert_eq!(headers, &["Name", "Age", "City"]);
        assert_eq!(rows.len(), 2);
    } else {
        panic!("expected Table block");
    }
}

// =========================================================================
// Whitespace-aligned tables
// =========================================================================

#[test]
fn whitespace_aligned_basic() {
    let doc = make_doc(vec![
        para("Name    Age    City"),
        para("Alice   30     NYC"),
        para("Bob     25     LA"),
    ]);
    let result = run_process(doc);
    assert_eq!(count_tables(&result), 1);
    let blocks: Vec<_> = result.iter_all_blocks().collect();
    if let PdfBlock::Table { headers, rows, .. } = blocks[0].1 {
        assert_eq!(headers, &["Name", "Age", "City"]);
        assert_eq!(rows.len(), 2);
    } else {
        panic!("expected Table block");
    }
}

// =========================================================================
// Non-table text not mis-detected
// =========================================================================

#[test]
fn regular_text_not_detected() {
    let doc = make_doc(vec![
        para("This is a normal paragraph."),
        para("It has multiple sentences. And some more text."),
        para("No table patterns here at all."),
    ]);
    let result = run_process(doc);
    assert_eq!(count_tables(&result), 0);
    let blocks: Vec<_> = result.iter_all_blocks().collect();
    assert_eq!(blocks.len(), 3);
    for (_, block) in &blocks {
        assert!(matches!(block, PdfBlock::Paragraph { .. }));
    }
}

#[test]
fn single_pipe_not_detected() {
    let doc = make_doc(vec![para("a | b")]);
    let result = run_process(doc);
    assert_eq!(count_tables(&result), 0);
}

#[test]
fn empty_document() {
    let doc = PdfDocumentModel::new(PdfMetadata::default(), Vec::new());
    let result = run_process(doc);
    assert_eq!(count_tables(&result), 0);
    assert!(result.is_empty());
}

// =========================================================================
// Min rows / min columns filtering
// =========================================================================

#[test]
fn below_min_columns_rejected() {
    let config = TableDetectionConfig::new().with_min_columns(3);
    let proc = TableDetectorProcessor::with_config(config);
    let doc = make_doc(vec![para("| A | B |"), para("| 1 | 2 |")]);
    let (result, _) = proc.process(&empty_input(), doc).unwrap();
    assert_eq!(count_tables(&result), 0);
}

#[test]
fn below_min_rows_rejected() {
    let config = TableDetectionConfig::new().with_min_rows(4);
    let proc = TableDetectorProcessor::with_config(config);
    let doc = make_doc(vec![
        para("| A | B |"),
        para("| 1 | 2 |"),
        para("| 3 | 4 |"),
    ]);
    let (result, _) = proc.process(&empty_input(), doc).unwrap();
    // 3 rows < min_rows(4)
    assert_eq!(count_tables(&result), 0);
}

#[test]
fn exact_min_rows_accepted() {
    let config = TableDetectionConfig::new().with_min_rows(3);
    let proc = TableDetectorProcessor::with_config(config);
    let doc = make_doc(vec![
        para("| A | B |"),
        para("| 1 | 2 |"),
        para("| 3 | 4 |"),
    ]);
    let (result, _) = proc.process(&empty_input(), doc).unwrap();
    assert_eq!(count_tables(&result), 1);
}

// =========================================================================
// Irregular tables
// =========================================================================

#[test]
fn irregular_table_rejected_by_default() {
    let doc = make_doc(vec![
        para("| A | B | C |"),
        para("| 1 | 2 |"), // 2 cols vs 3 — breaks scan, only header remains
        para("| 4 | 5 | 6 |"),
    ]);
    let result = run_process(doc);
    // The 2-col row breaks the scan after just the header (1 row < min_rows 2),
    // so no table is detected — all blocks remain as paragraphs.
    assert_eq!(count_tables(&result), 0);
}

#[test]
fn irregular_table_allowed() {
    let config = TableDetectionConfig::new().allow_irregular();
    let proc = TableDetectorProcessor::with_config(config);
    let doc = make_doc(vec![
        para("| A | B | C |"),
        para("| 1 | 2 |"),
        para("| 4 | 5 | 6 |"),
    ]);
    let (result, _) = proc.process(&empty_input(), doc).unwrap();
    assert_eq!(count_tables(&result), 1);
    let blocks: Vec<_> = result.iter_all_blocks().collect();
    if let PdfBlock::Table { rows, .. } = blocks[0].1 {
        assert_eq!(rows.len(), 2); // both data rows included
    } else {
        panic!("expected Table");
    }
}

// =========================================================================
// Multiple tables on same page
// =========================================================================

#[test]
fn multiple_tables_on_same_page() {
    let doc = make_doc(vec![
        para("| A | B |"),
        para("| 1 | 2 |"),
        para("Some text between tables."),
        para("| X | Y |"),
        para("| 3 | 4 |"),
    ]);
    let result = run_process(doc);
    assert_eq!(count_tables(&result), 2);
}

// =========================================================================
// Mixed block types
// =========================================================================

#[test]
fn heading_and_code_not_participating() {
    let doc = make_doc(vec![
        PdfBlock::heading(1, "Title", loc()),
        para("| A | B |"),
        para("| 1 | 2 |"),
        PdfBlock::code("fn main() {}", loc()),
    ]);
    let result = run_process(doc);
    let blocks: Vec<_> = result.iter_all_blocks().collect();
    assert_eq!(blocks.len(), 3);
    assert!(matches!(blocks[0].1, PdfBlock::Heading { .. }));
    assert!(matches!(blocks[1].1, PdfBlock::Table { .. }));
    assert!(matches!(blocks[2].1, PdfBlock::Code { .. }));
}

// =========================================================================
// ProcessorPipeline integration
// =========================================================================

#[test]
fn integrates_into_pipeline() {
    let mut pipeline = ProcessorPipeline::new();
    pipeline.register(Box::new(TableDetectorProcessor::new()));
    assert_eq!(pipeline.len(), 1);

    let doc = make_doc(vec![para("| A | B |"), para("| 1 | 2 |")]);
    let (result, warnings) = pipeline.run(&empty_input(), doc).unwrap();
    assert!(warnings.is_empty());
    assert_eq!(count_tables(&result), 1);
}

// =========================================================================
// Capability declaration
// =========================================================================

#[test]
fn capabilities_declare_table_detection() {
    let proc = TableDetectorProcessor::new();
    let caps = proc.capabilities();
    assert!(caps.table_detection());
    assert!(!caps.ocr());
    assert!(!caps.image_extraction());
}

// =========================================================================
// Page dimensions preserved
// =========================================================================

#[test]
fn page_dimensions_preserved() {
    let page = PdfPageModel::new(PageIndex::new(0))
        .with_dimensions(595.0, 842.0)
        .with_rotation(90)
        .with_block(para("| A | B |"))
        .with_block(para("| 1 | 2 |"));
    let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);

    let proc = TableDetectorProcessor::new();
    let (result, _) = proc.process(&empty_input(), doc).unwrap();

    let page = &result.pages()[0];
    assert_eq!(page.width_pt(), Some(595.0));
    assert_eq!(page.height_pt(), Some(842.0));
    assert_eq!(page.rotation(), 90);
}

// =========================================================================
// Source location carried to Table block
// =========================================================================

#[test]
fn source_location_from_first_row() {
    let special_loc = SourceLocation::new(PageIndex::new(3), 0.95);
    let page = PdfPageModel::new(PageIndex::new(3))
        .with_block(PdfBlock::paragraph("| A | B |", special_loc))
        .with_block(PdfBlock::paragraph("| 1 | 2 |", loc()));
    let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);

    let proc = TableDetectorProcessor::new();
    let (result, _) = proc.process(&empty_input(), doc).unwrap();

    let blocks: Vec<_> = result.iter_all_blocks().collect();
    if let PdfBlock::Table { source, .. } = blocks[0].1 {
        assert_eq!(source.page_index().value(), 3);
        assert!((source.confidence() - 0.95).abs() < f32::EPSILON);
    } else {
        panic!("expected Table");
    }
}

// =========================================================================
// Pipe-only / tab-only / whitespace-only config
// =========================================================================

#[test]
fn pipe_only_config_ignores_tabs() {
    let config = TableDetectionConfig::new().with_separator(ColumnSeparator::Pipe);
    let proc = TableDetectorProcessor::with_config(config);
    let doc = make_doc(vec![para("Name\tAge"), para("Alice\t30")]);
    let (result, _) = proc.process(&empty_input(), doc).unwrap();
    assert_eq!(count_tables(&result), 0);
}

#[test]
fn tab_only_config_ignores_pipes() {
    let config = TableDetectionConfig::new().with_separator(ColumnSeparator::Tab);
    let proc = TableDetectorProcessor::with_config(config);
    let doc = make_doc(vec![para("| Name | Age |"), para("| Alice | 30 |")]);
    let (result, _) = proc.process(&empty_input(), doc).unwrap();
    assert_eq!(count_tables(&result), 0);
}

#[test]
fn whitespace_only_config_ignores_pipes() {
    let config = TableDetectionConfig::new().with_separator(ColumnSeparator::Whitespace);
    let proc = TableDetectorProcessor::with_config(config);
    let doc = make_doc(vec![para("| Name | Age |"), para("| Alice | 30 |")]);
    let (result, _) = proc.process(&empty_input(), doc).unwrap();
    assert_eq!(count_tables(&result), 0);
}
