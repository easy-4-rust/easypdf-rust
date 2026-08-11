//! Integration test: all prelude items are importable.

use easypdf::prelude::*;

// --- Core types ---
#[allow(dead_code)]
fn assert_core_types() {
    // EasyPdf facade
    let _ = EasyPdf::create("/tmp/test.pdf");

    // PdfModel derive
    #[derive(PdfModel)]
    struct Dummy {
        #[pdf(text, position = (0.0, 0.0))]
        name: String,
    }

    // Core enums
    let _: PageSize = PageSize::A4;
    let _: Orientation = Orientation::Portrait;
    let _: Rotation = Rotation::Clockwise90;

    // Core structs
    let _ = PdfText::new("hello");
    let _ = PdfFont::helvetica(12.0);
    let _ = PdfMetadata::default();

    // Model types
    let _: PdfBlock;
    let _: PdfBlockType;

    // I/O types
    let _: ResourceLimits;
}

// --- Markdown types (behind feature gate) ---
#[cfg(feature = "markdown")]
#[allow(dead_code)]
fn assert_markdown_types() {
    let _: MarkdownProfile;
    let _: ProcessorPipeline;
    let _: ImagePolicy;
    let _: OcrPolicy;
    let _: TablePolicy;
}

// --- Table detection types (behind feature gate) ---
#[cfg(feature = "markdown-table")]
#[allow(dead_code)]
fn assert_table_types() {
    let _: TableDetectorProcessor;
    let _: TableDetectionConfig;
}

// --- OCR types (behind feature gate) ---
#[cfg(feature = "ocr")]
#[allow(dead_code)]
fn assert_ocr_types() {
    let _: OcrConfig;
    let _: OcrTrigger;
}

// --- Render types (behind feature gate) ---
#[cfg(feature = "render")]
#[allow(dead_code)]
fn assert_render_types() {
    let _: RenderBackend;
    let _: RenderConfig;
}

// --- Resident types (behind feature gate) ---
#[cfg(feature = "resident")]
#[allow(dead_code)]
fn assert_resident_types() {
    let _: AutosaveMode;
    let _: ResidentServer;
    let _: ResidentClient;
}

// --- MCP types (behind feature gate) ---
#[cfg(feature = "mcp")]
#[allow(dead_code)]
fn assert_mcp_types() {
    let _ = McpServer::new();
}
