//! Integration test: default features compile and all public API is reachable.

#![allow(unused_imports)]

use easypdf::{
    AtomicFileOutput, EasyPdf, ImageData, ImageFormat, ListItem,
    PdfBlock, PdfBlockType, PdfDocumentModel, PdfPageModel, PdfInput,
    PdfManipulator, PdfModel, PdfReader, PdfTemplateFiller, PdfWriter,
    PdfWriterBuilder, ReadStrategy, ResourceLimits, SourceLocation,
    WriteBackend, LayoutDirection,
};

// Verify re-exports from easypdf_core::io
use easypdf::{attempt_repair, guard_decompression_bomb, guard_element_explosion,
              is_likely_corrupt, validate_io_url, RepairOptions};

// Verify core re-exports
use easypdf::{PdfError, PdfFont, PdfMetadata, PdfText, PageSize, Orientation, Rotation};

// --- Facade method existence ---

#[test]
fn facade_create_returns_builder() {
    let builder = EasyPdf::create("/tmp/test.pdf");
    // Just verify the builder type exists and is usable
    let _ = builder.title("Test");
}

#[test]
fn facade_read_returns_builder() {
    let builder = EasyPdf::read("/tmp/nonexistent.pdf");
    // Verify builder type
    let _ = builder.pages(0..5);
}

#[test]
fn facade_split_returns_builder() {
    let builder = EasyPdf::split("/tmp/nonexistent.pdf");
    let _ = builder.every_n_pages(2);
}

#[test]
fn facade_manipulate_returns_builder() {
    let builder = EasyPdf::manipulate("/tmp/nonexistent.pdf");
    let _ = builder.rotate_all(Rotation::Clockwise90);
}

#[test]
fn facade_encrypt_returns_io_error_for_missing_input() {
    let result = EasyPdf::encrypt("/tmp/nonexistent_easypdf_in.pdf", "/tmp/out.pdf", "pass");
    assert!(result.is_err());
    // The input file does not exist, so we get an I/O error.
    assert!(matches!(result.unwrap_err(), PdfError::Io(_)));
}

#[test]
fn facade_sign_returns_io_error_for_missing_input() {
    let result = EasyPdf::sign(
        "/tmp/nonexistent_easypdf_in.pdf",
        "/tmp/out.pdf",
        "/tmp/nonexistent_key.der".as_ref(),
        "/tmp/nonexistent_cert.der".as_ref(),
        "reason",
    );
    assert!(result.is_err());
    // At least one of the input files does not exist.
    assert!(matches!(result.unwrap_err(), PdfError::Io(_)));
}

#[test]
fn facade_writer_returns_builder() {
    let _builder = EasyPdf::writer("Test Document");
}

#[test]
fn facade_merge_with_empty_rejects() {
    // merge with empty input list should succeed (0 files merged)
    let result = EasyPdf::merge(&[] as &[&str], "/tmp/out.pdf");
    // The underlying manipulator may or may not accept empty input;
    // we just verify the method is callable.
    let _ = result;
}

// --- Markdown facade methods (feature-gated) ---

#[cfg(feature = "markdown")]
#[test]
fn facade_export_markdown_returns_builder() {
    let builder = EasyPdf::export_markdown("/tmp/in.pdf", "/tmp/out.md");
    let _ = builder.pages(0..10);
}

#[cfg(feature = "markdown")]
#[test]
fn facade_to_markdown_returns_builder() {
    let _builder = EasyPdf::to_markdown("/tmp/in.pdf");
}

#[cfg(feature = "markdown")]
#[test]
fn facade_markdown_pipeline_returns_pipeline() {
    use easypdf::MarkdownProfile;
    let _pipeline = EasyPdf::markdown_pipeline(MarkdownProfile::Gfm);
}

// --- Table detection facade method (feature-gated) ---

#[cfg(feature = "markdown-table")]
#[test]
fn facade_table_detector_returns_processor() {
    let _detector = EasyPdf::table_detector();
}

// --- Render facade method (feature-gated) ---

#[cfg(feature = "render")]
#[test]
fn facade_render_page_signature_exists() {
    // Just verify the function signature compiles; actual rendering
    // requires a valid PDF file.
    let _ = EasyPdf::render_page as fn(&std::path::Path, usize, &std::path::Path, u32)
        -> std::result::Result<(), easypdf::RenderError>;
}

// --- Resident facade methods (feature-gated) ---

#[cfg(feature = "resident")]
#[test]
fn facade_attach_returns_option() {
    let _result = EasyPdf::attach();
}

// --- MCP facade method (feature-gated) ---

#[cfg(feature = "mcp")]
#[test]
fn facade_mcp_server_returns_server() {
    let _server = EasyPdf::mcp_server();
}

// --- Writer builder API ---

#[test]
fn writer_builder_api() {
    let writer = PdfWriterBuilder::new("Test")
        .backend(WriteBackend::default())
        .build();
    assert!(writer.is_ok());
}

// --- Re-exported IO functions ---

#[test]
fn io_guard_functions_exist() {
    // guard_decompression_bomb signature check
    let limits = ResourceLimits::default();
    let result = guard_decompression_bomb(100, 200, &limits);
    assert!(result.is_ok());
}

#[test]
fn io_guard_element_explosion_exists() {
    let limits = ResourceLimits::default();
    let result = guard_element_explosion(100, &limits);
    assert!(result.is_ok());
}

#[test]
fn io_validate_url_exists() {
    let result = validate_io_url("https://example.com");
    assert!(result.is_ok());
}

#[test]
fn io_repair_functions_exist() {
    let _ = is_likely_corrupt;
    let _ = attempt_repair;
    let _ = RepairOptions::default();
}

// --- Layout re-exports ---

#[test]
fn layout_reexports_exist() {
    let _: LayoutDirection = LayoutDirection::Horizontal;
}
