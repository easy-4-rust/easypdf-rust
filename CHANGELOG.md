# Changelog

## [0.1.0] — 2026-08-09

### Added
- 11-crate workspace: `easypdf`, `easypdf-core`, `easypdf-model`, `easypdf-io`, `easypdf-derive`, `easypdf-layout`, `easypdf-reader`, `easypdf-writer`, `easypdf-manipulate`, `easypdf-template`, `easypdf-markdown`
- Static factory `EasyPdf` with fluent builder API
- `#[derive(PdfModel)]` proc-macro for compile-time struct-to-PDF mapping
- PDF creation: text, built-in fonts (14 standard), metadata, pages (A4/Letter/Custom)
- PDF reading: text extraction, metadata extraction, single-parse session reuse (~129x speedup)
- PDF → Markdown: GFM/LLM/Plain profiles, zero-based page range, export report, structured warnings
- Merge: multiple PDFs into one with valid `/Pages` tree
- Split: PDF into individual pages with valid `/Pages` tree
- Rotate: per-page or all-page rotation (0°/90°/180°/270°)
- Reorder: arbitrary page reordering
- Template fill: AcroForm field replacement
- Writer lifecycle hooks: `PdfWriteHandler` with before/after document/page callbacks
- Event-driven read listeners: `PdfReadListener` trait
- Engine-neutral semantic IR: `PdfDocumentModel`, `PdfPageModel`, `PdfBlock` in `easypdf-model`
- Backend-neutral layout: `LayoutSink` trait in `easypdf-layout`, `FlowLayout`
- Atomic output: temp file + atomic rename for all save operations
- Resource limits: max file size (100 MB), max pages (10,000), max text length (10 MB)
- Type system: `PageSize`, `Orientation`, `Rotation`, `TextAlignment`, `PdfColor`, `PdfFont`, `BuiltInFont`
- Error handling: 7-variant `PdfError` enum with `thiserror`
- Bilingual README (EN/ZH), architecture design documents, usage guide
- Reader session benchmark: `cargo bench -p easypdf-reader --bench reader_session`
- 136 tests passing across all crates

### Known Limitations
- Encryption returns `UnsupportedFeature` (planned v0.4)
- Digital signatures return `UnsupportedFeature` (planned v0.5)
- Table detection, image extraction, OCR emit structured warnings in Markdown pipeline
- Custom TTF/OTF fonts: `register_font_from_path` exists but not fully integrated into all builders
