# easypdf-markdown

> PDF-to-Markdown conversion layer: deterministic pipeline, semantic processor chain, table detection, OCR abstraction, and page rendering.

## Role

`easypdf-markdown` converts PDF content deterministically into Markdown strings. It uses a `ProcessorPipeline` architecture where semantic processors (reading order, heading detection, link extraction, table detection, OCR fallback) run in priority order to enrich the document model before rendering. The pipeline flows: `PdfInput -> PdfReader -> PdfDocumentModel -> ProcessorPipeline -> MarkdownRenderer -> String`.

## Core Capabilities

- **Processor pipeline** (`ProcessorPipeline`) -- compose multiple semantic processors by priority (`crates/easypdf-markdown/src/processor_pipeline.rs`)
- **Markdown rendering** (`MarkdownRenderer`) -- convert `PdfDocumentModel` to Markdown string (`crates/easypdf-markdown/src/markdown_renderer.rs`)
- **Built-in processors** -- `ReadingOrderProcessor`, `HeadingDetectorProcessor`, `LinkExtractorProcessor` (`crates/easypdf-markdown/src/processors/`)
- **Table detection** (`TableDetectorProcessor`) -- heuristic table boundary identification with configurable `TableDetectionConfig` (`crates/easypdf-markdown/src/table/`)
- **OCR abstraction** (`OcrProcessor` + `OcrEngine` trait) -- pluggable OCR engine interface for fallback text extraction (`crates/easypdf-markdown/src/ocr/`)
- **Page rendering** (`PdfRenderer` trait) -- rasterize PDF pages to images; text backend by default, PDFium backend optional (`crates/easypdf-markdown/src/render/`)
- **Profile presets** (`MarkdownProfile`) -- `balanced` / `fast` / `high_quality` configuration presets via `MarkdownProfileBuilder` (`crates/easypdf-markdown/src/markdown_profile.rs`)
- **Export builders** -- `PdfMarkdownBuilder` (in-memory) and `PdfMarkdownExportBuilder` (file export) with `MarkdownExportReport` (`crates/easypdf-markdown/src/pdf_markdown_builder.rs`)

## Dependencies

### Internal

| Crate | Purpose |
|-------|---------|
| `easypdf-core` | Core types (`PdfDocumentModel`, `PdfPageModel`, `PdfBlock`) |
| `easypdf-reader` | PDF parsing and text extraction |

### External

| Crate | Version | Purpose |
|-------|---------|---------|
| `lopdf` | 0.44.0 | PDF object model |
| `image` | 0.25.9 | Image processing |
| `serde` / `serde_json` | 1.x | Serialization |

## Main API

### PdfMarkdownBuilder (in-memory conversion)

```rust
use easypdf_markdown::{PdfMarkdownBuilder, MarkdownProfile};

let result = PdfMarkdownBuilder::new("document.pdf")
    .profile(MarkdownProfile::balanced())
    .build()
    .convert_to_markdown()?;

println!("{}", result.markdown);
```

### PdfMarkdownExportBuilder (file export)

```rust
use easypdf_markdown::PdfMarkdownExportBuilder;

let report = PdfMarkdownExportBuilder::new("document.pdf")
    .output_path("output.md")
    .build()
    .export()?;
```

### ProcessorPipeline

```rust
use easypdf_markdown::{
    ProcessorPipeline,
    processors::{ReadingOrderProcessor, HeadingDetectorProcessor, LinkExtractorProcessor},
};

let mut pipeline = ProcessorPipeline::new();
pipeline.register(Box::new(ReadingOrderProcessor));
pipeline.register(Box::new(HeadingDetectorProcessor::new()));
pipeline.register(Box::new(LinkExtractorProcessor::new()));
```

### TableDetectorProcessor

```rust
use easypdf_markdown::{TableDetectorProcessor, TableDetectionConfig};

let detector = TableDetectorProcessor::new(TableDetectionConfig::default());
pipeline.register(Box::new(detector));
```

### OcrProcessor

```rust
use easypdf_markdown::{OcrProcessor, OcrConfig, OcrTrigger};

let config = OcrConfig::default().trigger(OcrTrigger::LowConfidence);
let ocr = OcrProcessor::new(config);
pipeline.register(Box::new(ocr));
```

### Page Rendering

```rust
use easypdf_markdown::render::{render_page_to_png, RenderConfig};

let image = render_page_to_png("document.pdf", 0, &RenderConfig::default())?;
```

## Feature Flags

| Feature | Dependencies | Description |
|---------|-------------|-------------|
| `pdfium` | `pdfium-render: 0.8` | High-quality PDFium rendering backend (requires `libpdfium`) |
| `ocrs` | `ocrs: 0.9` | Pure-Rust OCR engine |
| `llm` | `rig-core: 0.8`, `base64`, `tokio` | LLM Vision OCR (OpenAI / Gemini / DeepSeek) |

## License

Apache-2.0

---

**Project**: https://github.com/easy-4-rust/easypdf-rust
**crates.io**: https://crates.io/crates/easypdf-markdown
**docs.rs**: https://docs.rs/easypdf-markdown
